use super::*;

pub(crate) async fn callback_shipment(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shipment_id = match uuid::Uuid::parse_str(&shipment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let shipments_repo = ShipmentsRepo::new(state.db.pool());
    let shipment = match shipments_repo.get_by_id(tenant_id, shipment_id).await {
        Ok(Some(shipment)) => shipment,
        Ok(None) => return not_found("shipment not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let carrier_code = match shipment.carrier_code.clone().and_then(non_empty) {
        Some(value) => value,
        None => return bad_request("carrier_code required").into_response(),
    };
    let tracking_no = match shipment.tracking_no.clone().and_then(non_empty) {
        Some(value) => value,
        None => return bad_request("tracking_no required").into_response(),
    };
    let shipped_at = shipment.shipped_at.or(Some(Utc::now()));
    let warehouse_id = match shipment.warehouse_id {
        Some(value) => value,
        None => return bad_request("warehouse_id required").into_response(),
    };
    let payload = ShipmentShippedPayload {
        tenant_id,
        shipment_id: shipment.shipment_id,
        order_id: OrderId(shipment.order_id),
        shop_id: ShopId(shipment.shop_id),
        platform: Platform::from_str(&shipment.platform).unwrap_or(Platform::Taobao),
        platform_order_id: shipment.platform_order_id.clone(),
        warehouse_id,
        carrier_code: Some(carrier_code.clone()),
        tracking_no: Some(tracking_no.clone()),
        shipped_at,
    };
    let outbox_repo = OutboxRepo::new(state.db.pool());
    if let Ok(out_payload) = serde_json::to_value(payload) {
        let dedup_key = format!("SHIPMENT_CALLBACK:{}", shipment.shipment_id);
        if let Err(err) = outbox_repo
            .enqueue_dedup_with_trace(
                tenant_id,
                SHIPMENT_SHIPPED_EVENT,
                &out_payload,
                &dedup_key,
                Some(req_ctx.trace_id.as_str()),
            )
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
    }

    let log_metadata = serde_json::json!({
        "shipment_id": shipment.shipment_id,
        "order_id": shipment.order_id,
        "warehouse_id": warehouse_id,
        "carrier_code": carrier_code,
        "tracking_no": tracking_no,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_SHIPMENT_CALLBACK.to_string(),
            ref_type: Some("SHIPMENT".to_string()),
            ref_id: Some(shipment.shipment_id.to_string()),
            metadata: Some(log_metadata.clone()),
        })
        .await;
    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SHIPMENT.CALLBACK",
        "shipment",
        Some(shipment.shipment_id.to_string()),
        None,
        Some(log_metadata),
    )
    .await;

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "queued" })),
    )
        .into_response()
}

pub(crate) async fn confirm_shipment(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shipment_id = match uuid::Uuid::parse_str(&shipment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let shipments_repo = ShipmentsRepo::new(state.db.pool());
    let shipment = match shipments_repo.get_by_id(tenant_id, shipment_id).await {
        Ok(Some(shipment)) => shipment,
        Ok(None) => return not_found("shipment not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let packages = match shipments_repo.list_packages(tenant_id, shipment_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let carrier_code = shipment.carrier_code.clone().and_then(non_empty);
    let tracking_no = shipment.tracking_no.clone().and_then(non_empty);
    let has_package_carrier = packages.iter().any(|pkg| {
        pkg.carrier_code
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    });
    let has_package_tracking = packages.iter().any(|pkg| {
        pkg.tracking_no
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    });
    if carrier_code.is_none() && !has_package_carrier {
        return bad_request("carrier_code required").into_response();
    }
    if tracking_no.is_none() && !has_package_tracking {
        return bad_request("tracking_no required").into_response();
    }

    let shipped_at = shipment.shipped_at.or(Some(Utc::now()));
    let warehouse_id = match shipment.warehouse_id {
        Some(value) => value,
        None => return bad_request("warehouse_id required").into_response(),
    };
    let payload = ShipmentShippedPayload {
        tenant_id,
        shipment_id: shipment.shipment_id,
        order_id: OrderId(shipment.order_id),
        shop_id: ShopId(shipment.shop_id),
        platform: Platform::from_str(&shipment.platform).unwrap_or(Platform::Taobao),
        platform_order_id: shipment.platform_order_id.clone(),
        warehouse_id,
        carrier_code: carrier_code.clone(),
        tracking_no: tracking_no.clone(),
        shipped_at,
    };
    let outbox_repo = OutboxRepo::new(state.db.pool());
    if let Ok(out_payload) = serde_json::to_value(payload) {
        let dedup_key = format!("SHIPMENT_CONFIRM:{}", shipment.shipment_id);
        if let Err(err) = outbox_repo
            .enqueue_dedup_with_trace(
                tenant_id,
                SHIPMENT_CONFIRMED_EVENT,
                &out_payload,
                &dedup_key,
                Some(req_ctx.trace_id.as_str()),
            )
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
    }
    if let Err(err) = shipments_repo
        .update_push_status(tenant_id, shipment_id, "SHIPPING", "")
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let log_metadata = serde_json::json!({
        "shipment_id": shipment.shipment_id,
        "order_id": shipment.order_id,
        "warehouse_id": warehouse_id,
        "carrier_code": carrier_code.clone(),
        "tracking_no": tracking_no.clone(),
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_SHIPMENT_CONFIRMED.to_string(),
            ref_type: Some("SHIPMENT".to_string()),
            ref_id: Some(shipment.shipment_id.to_string()),
            metadata: Some(log_metadata.clone()),
        })
        .await;
    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SHIPMENT.CONFIRM",
        "shipment",
        Some(shipment.shipment_id.to_string()),
        None,
        Some(log_metadata),
    )
    .await;

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "queued" })),
    )
        .into_response()
}

pub(crate) async fn list_shipment_packages(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shipment_id = match uuid::Uuid::parse_str(&shipment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let shipments_repo = ShipmentsRepo::new(state.db.pool());
    let exists = match shipments_repo.get_by_id(tenant_id, shipment_id).await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if !exists {
        return not_found("shipment not found").into_response();
    }

    let packages = match shipments_repo.list_packages(tenant_id, shipment_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let items = packages
        .into_iter()
        .map(|pkg| ShipmentPackageResponse {
            package_id: pkg.package_id,
            shipment_id: pkg.shipment_id,
            package_no: pkg.package_no,
            carrier_code: pkg.carrier_code,
            tracking_no: pkg.tracking_no,
            weight_gram: pkg.weight_gram,
            created_at: pkg.created_at,
            updated_at: pkg.updated_at,
        })
        .collect();

    (StatusCode::OK, Json(ShipmentPackageListResponse { items })).into_response()
}

pub(crate) async fn upsert_shipment_packages(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShipmentPackageUpsertRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shipment_id = match uuid::Uuid::parse_str(&shipment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let shipments_repo = ShipmentsRepo::new(state.db.pool());
    let shipment = match shipments_repo.get_by_id(tenant_id, shipment_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shipment not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let before_packages = match shipments_repo.list_packages(tenant_id, shipment_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut packages = Vec::with_capacity(req.packages.len());
    for entry in req.packages {
        let package_no = entry.package_no.trim();
        if package_no.is_empty() {
            return bad_request("package_no required").into_response();
        }
        if let Some(weight_gram) = entry.weight_gram {
            if weight_gram <= 0 {
                return bad_request("weight_gram must be positive").into_response();
            }
        }
        packages.push(ShipmentPackageUpsertInput {
            package_no: package_no.to_string(),
            carrier_code: entry.carrier_code.and_then(non_empty),
            tracking_no: entry.tracking_no.and_then(non_empty),
            weight_gram: entry.weight_gram,
        });
    }

    let saved = match shipments_repo
        .upsert_packages(tenant_id, shipment_id, packages)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    if !saved.is_empty() {
        if let Ok(Some(state_row)) = orders_repo
            .get_order_review_state(tenant_id, OrderId(shipment.order_id))
            .await
        {
            if !matches!(
                state_row.fulfillment_status,
                OrderFulfillmentStatus::Shipped | OrderFulfillmentStatus::Signed
            ) && state_row.fulfillment_status != OrderFulfillmentStatus::ReadyToShip
            {
                let next_fulfillment_status = OrderFulfillmentStatus::ReadyToShip;
                let next_order_status = derive_order_status(
                    state_row.oms_status,
                    state_row.review_status,
                    next_fulfillment_status,
                    state_row.exception_status,
                );
                let _ = orders_repo
                    .update_order_review_state(
                        tenant_id,
                        OrderId(shipment.order_id),
                        OrderReviewUpdate {
                            oms_status: state_row.oms_status,
                            order_status: next_order_status,
                            review_status: state_row.review_status,
                            fulfillment_status: next_fulfillment_status,
                            exception_status: state_row.exception_status,
                            exception_flags: state_row.exception_flags.clone(),
                            expected_version: Some(state_row.version),
                        },
                    )
                    .await;
            }
        }
    }

    let before_snapshot = before_packages
        .iter()
        .map(|pkg| ShipmentPackageResponse {
            package_id: pkg.package_id,
            shipment_id: pkg.shipment_id,
            package_no: pkg.package_no.clone(),
            carrier_code: pkg.carrier_code.clone(),
            tracking_no: pkg.tracking_no.clone(),
            weight_gram: pkg.weight_gram,
            created_at: pkg.created_at,
            updated_at: pkg.updated_at,
        })
        .collect::<Vec<_>>();
    let after_snapshot = saved
        .iter()
        .map(|pkg| ShipmentPackageResponse {
            package_id: pkg.package_id,
            shipment_id: pkg.shipment_id,
            package_no: pkg.package_no.clone(),
            carrier_code: pkg.carrier_code.clone(),
            tracking_no: pkg.tracking_no.clone(),
            weight_gram: pkg.weight_gram,
            created_at: pkg.created_at,
            updated_at: pkg.updated_at,
        })
        .collect::<Vec<_>>();

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SHIPMENT.PACKAGES_UPSERT".to_string(),
            entity_type: "shipment".to_string(),
            entity_id: Some(shipment_id.to_string()),
            before_json: Some(serde_json::to_value(before_snapshot).unwrap_or_default()),
            after_json: Some(serde_json::to_value(after_snapshot).unwrap_or_default()),
            ip: None,
            ua: None,
        })
        .await;
    let items = saved
        .into_iter()
        .map(|pkg| ShipmentPackageResponse {
            package_id: pkg.package_id,
            shipment_id: pkg.shipment_id,
            package_no: pkg.package_no,
            carrier_code: pkg.carrier_code,
            tracking_no: pkg.tracking_no,
            weight_gram: pkg.weight_gram,
            created_at: pkg.created_at,
            updated_at: pkg.updated_at,
        })
        .collect();

    (StatusCode::OK, Json(ShipmentPackageListResponse { items })).into_response()
}

pub(crate) async fn list_logistics_match_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsMatchRuleListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsMatchRuleRead.as_str())
    {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let repo = LogisticsMatchRulesRepo::new(state.db.pool());
    let result = match repo
        .list_rules(
            tenant_id,
            LogisticsMatchRuleListFilters {
                keyword,
                enabled: query.enabled,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsMatchRuleListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_logistics_match_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsMatchRuleRead.as_str())
    {
        return resp.into_response();
    }

    let repo = LogisticsMatchRulesRepo::new(state.db.pool());
    let rule = match repo.get_rule(tenant_id, rule_id).await {
        Ok(Some(rule)) => rule,
        Ok(None) => return not_found("logistics match rule not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn create_logistics_match_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsMatchRuleCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) =
        ensure_permission(&ctx, PermissionCode::BasicLogisticsMatchRuleWrite.as_str())
    {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }

    let product_id = match req.product_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad product_id").into_response(),
        },
        None => None,
    };

    let repo = LogisticsMatchRulesRepo::new(state.db.pool());
    let rule = match repo
        .create_rule(LogisticsMatchRuleCreate {
            tenant_id,
            name: name.to_string(),
            enabled: req.enabled.unwrap_or(true),
            priority: req.priority.unwrap_or(0),
            product_id,
            conditions_json: req.conditions_json.unwrap_or_else(|| serde_json::json!([])),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "LOGISTICS_MATCH_RULE.CREATE".to_string(),
            entity_type: "logistics_match_rule".to_string(),
            entity_id: Some(rule.rule_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&rule).unwrap_or(Value::Null)),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn update_logistics_match_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsMatchRuleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) =
        ensure_permission(&ctx, PermissionCode::BasicLogisticsMatchRuleWrite.as_str())
    {
        return resp.into_response();
    }

    let product_id = match req.product_id {
        Some(Some(raw)) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => Some(Some(v)),
            Err(_) => return bad_request("bad product_id").into_response(),
        },
        Some(None) => Some(None),
        None => None,
    };

    let repo = LogisticsMatchRulesRepo::new(state.db.pool());
    let rule = match repo
        .update_rule(
            tenant_id,
            rule_id,
            LogisticsMatchRuleUpdate {
                name: req
                    .name
                    .map(|value| value.trim().to_string())
                    .filter(|v| !v.is_empty()),
                enabled: req.enabled,
                priority: req.priority,
                product_id,
                conditions_json: req.conditions_json,
            },
        )
        .await
    {
        Ok(Some(rule)) => rule,
        Ok(None) => return not_found("logistics match rule not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "LOGISTICS_MATCH_RULE.UPDATE".to_string(),
            entity_type: "logistics_match_rule".to_string(),
            entity_id: Some(rule.rule_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&rule).unwrap_or(Value::Null)),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn delete_logistics_match_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) =
        ensure_permission(&ctx, PermissionCode::BasicLogisticsMatchRuleWrite.as_str())
    {
        return resp.into_response();
    }

    let repo = LogisticsMatchRulesRepo::new(state.db.pool());
    let rule = match repo.delete_rule(tenant_id, rule_id).await {
        Ok(Some(rule)) => rule,
        Ok(None) => return not_found("logistics match rule not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "LOGISTICS_MATCH_RULE.DELETE".to_string(),
            entity_type: "logistics_match_rule".to_string(),
            entity_id: Some(rule.rule_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&rule).unwrap_or(Value::Null)),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn list_logistics_match_hits(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsMatchHitListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsMatchRuleRead.as_str())
    {
        return resp.into_response();
    }

    let rule_id = match query.rule_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad rule_id").into_response(),
        },
        None => None,
    };
    let shipment_id = match query.shipment_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shipment_id").into_response(),
        },
        None => None,
    };
    let order_id = match query.order_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad order_id").into_response(),
        },
        None => None,
    };

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = LogisticsMatchHitsRepo::new(state.db.pool());
    let result = match repo
        .list_hits(
            tenant_id,
            LogisticsMatchHitListFilters {
                rule_id,
                shipment_id,
                order_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsMatchHitListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn preview_logistics_match(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsMatchPreviewRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let order_id = match OrderId::from_str(&req.order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsMatchRuleRead.as_str())
    {
        return resp.into_response();
    }

    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(detail)) => detail,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let input = LogisticsMatchInput {
        warehouse_id,
        weight_gram: req.weight_gram,
    };
    let match_result = match evaluate_logistics_match(&state, tenant_id, &detail, &input).await {
        Ok(v) => v,
        Err(e) => {
            warn!(tenant_id = %tenant_id, order_id = %order_id, error = %e, "logistics match preview failed");
            None
        }
    };

    if let Some(match_result) = match_result.as_ref() {
        let hits_repo = LogisticsMatchHitsRepo::new(state.db.pool());
        let result_json = serde_json::json!({
            "rule_name": match_result.rule.name,
            "product_id": match_result.product.as_ref().map(|product| product.product_id),
            "carrier_code": match_result.product.as_ref().map(|product| product.carrier_code.clone()),
            "warehouse_id": warehouse_id,
            "weight_gram": req.weight_gram,
            "trigger": "PREVIEW"
        });
        if let Err(err) = hits_repo
            .insert_hit(LogisticsMatchHitCreate {
                tenant_id,
                rule_id: Some(match_result.rule.rule_id),
                shipment_id: None,
                order_id: Some(uuid::Uuid::from(order_id)),
                product_id: match_result
                    .product
                    .as_ref()
                    .map(|product| product.product_id),
                hit_reason: Some(match_result.rule.name.clone()),
                result_json: Some(result_json),
                success: true,
            })
            .await
        {
            warn!(error = %err, "logistics match preview hit insert failed");
        }
    }

    let (rule, product) = match match_result {
        Some(result) => (Some(result.rule), result.product),
        None => (None, None),
    };
    let response = LogisticsMatchPreviewResponse {
        matched: rule.is_some(),
        rule,
        product,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_logistics_shipments(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsShipmentListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let tracking_no = query
        .tracking_no
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let repo = ShipmentsRepo::new(state.db.pool());
    let result = match repo
        .list_shipments(
            tenant_id,
            ShipmentListFilters {
                shop_id: None,
                shop_ids: None,
                status: None,
                platform: None,
                shipment_type: None,
                tracking_no,
                updated_from: None,
                updated_to: None,
                limit,
                offset,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let total = result.items.len() as i64;
    let items = result
        .items
        .into_iter()
        .map(|shipment| LogisticsShipmentSummary {
            shipment_id: shipment.shipment_id,
            carrier_code: shipment.carrier_code,
            tracking_no: shipment.tracking_no,
            status: shipment.status,
            weight_gram: shipment.weight_gram,
            updated_at: shipment.updated_at,
        })
        .collect();

    let response = LogisticsShipmentListResponse { items, total };

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_shipment_tracking(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let shipment_id = match uuid::Uuid::parse_str(&shipment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };

    let repo = ShipmentsRepo::new(state.db.pool());
    let shipment = match repo.get_by_id(tenant_id, shipment_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shipment not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    #[derive(sqlx::FromRow)]
    struct TrackingRow {
        status: String,
        description: Option<String>,
        occurred_at: DateTime<Utc>,
    }

    let tracking_row: Option<TrackingRow> = match sqlx::query_as(
        r#"
        select status, description, occurred_at
        from shipment_tracking_events
        where tenant_id = $1 and shipment_id = $2
        order by occurred_at desc, created_at desc
        limit 1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let (latest_status, latest_description, latest_at) = match tracking_row {
        Some(row) => (row.status, row.description, row.occurred_at),
        None => (shipment.status.clone(), None, shipment.updated_at),
    };

    let response = ShipmentTrackingResponse {
        shipment_id: shipment.shipment_id,
        carrier_code: shipment.carrier_code,
        tracking_no: shipment.tracking_no,
        latest_status,
        latest_description,
        latest_at,
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_logistics_interfaces(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsInterfaceListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let approval_status = query
        .approval_status
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let repo = LogisticsInterfacesRepo::new(state.db.pool());
    let result = match repo
        .list_interfaces(
            tenant_id,
            LogisticsInterfaceListFilters {
                keyword,
                status,
                approval_status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsInterfaceListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_logistics_interface(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsInterfaceCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let carrier_code = req.carrier_code.trim();
    if carrier_code.is_empty() {
        return bad_request("carrier_code required").into_response();
    }
    let carrier_name = req.carrier_name.trim();
    if carrier_name.is_empty() {
        return bad_request("carrier_name required").into_response();
    }
    let api_base_url = req.api_base_url.trim();
    if api_base_url.is_empty() {
        return bad_request("api_base_url required").into_response();
    }
    let template_name = req.template_name.trim();
    if template_name.is_empty() {
        return bad_request("template_name required").into_response();
    }
    let retry_policy = req.retry_policy.trim();
    if retry_policy.is_empty() {
        return bad_request("retry_policy required").into_response();
    }
    let alert_channel = req.alert_channel.trim();
    if alert_channel.is_empty() {
        return bad_request("alert_channel required").into_response();
    }

    let status = req.status.unwrap_or_else(|| "HEALTHY".to_string());
    let approval_status = req.approval_status.unwrap_or_else(|| "PENDING".to_string());

    let repo = LogisticsInterfacesRepo::new(state.db.pool());
    let interface = match repo
        .create_interface(LogisticsInterfaceCreate {
            tenant_id,
            carrier_code: carrier_code.to_string(),
            carrier_name: carrier_name.to_string(),
            api_base_url: api_base_url.to_string(),
            template_name: template_name.to_string(),
            status,
            last_checked_at: req.last_checked_at,
            retry_policy: retry_policy.to_string(),
            alert_channel: alert_channel.to_string(),
            approval_status,
            approved_by: req.approved_by,
            approved_at: req.approved_at,
            notes: req.notes.filter(|value| !value.trim().is_empty()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(interface)).into_response()
}

pub(crate) async fn update_logistics_interface(
    State(state): State<AppState>,
    Path((tenant_id, interface_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsInterfaceUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let interface_id = match uuid::Uuid::parse_str(&interface_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad interface_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let carrier_code = req
        .carrier_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let carrier_name = req
        .carrier_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let api_base_url = req
        .api_base_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let template_name = req
        .template_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let retry_policy = req
        .retry_policy
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let alert_channel = req
        .alert_channel
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let approval_status = req
        .approval_status
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let approved_by = req.approved_by.map(|value| {
        value.and_then(|v| {
            let trimmed = v.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    });
    let notes = req.notes.map(|value| {
        value.and_then(|v| {
            let trimmed = v.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    });

    let repo = LogisticsInterfacesRepo::new(state.db.pool());
    let interface = match repo
        .update_interface(
            tenant_id,
            interface_id,
            LogisticsInterfaceUpdate {
                carrier_code,
                carrier_name,
                api_base_url,
                template_name,
                status: req.status,
                last_checked_at: req.last_checked_at,
                retry_policy,
                alert_channel,
                approval_status,
                approved_by,
                approved_at: req.approved_at,
                notes,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("logistics interface not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(interface)).into_response()
}

pub(crate) async fn list_logistics_interface_approvals(
    State(state): State<AppState>,
    Path((tenant_id, interface_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let interface_id = match uuid::Uuid::parse_str(&interface_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad interface_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let repo = LogisticsInterfacesRepo::new(state.db.pool());
    let approvals = match repo.list_approvals(tenant_id, interface_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(approvals)).into_response()
}

pub(crate) async fn create_logistics_interface_approval(
    State(state): State<AppState>,
    Path((tenant_id, interface_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsInterfaceApprovalCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let interface_id = match uuid::Uuid::parse_str(&interface_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad interface_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let action = req.action.trim();
    if action.is_empty() {
        return bad_request("action required").into_response();
    }
    let operator = req.operator.trim();
    if operator.is_empty() {
        return bad_request("operator required").into_response();
    }

    let repo = LogisticsInterfacesRepo::new(state.db.pool());
    let approval = match repo
        .insert_approval(LogisticsInterfaceApprovalCreate {
            tenant_id,
            interface_id,
            action: action.to_string(),
            operator: operator.to_string(),
            comment: req.comment.filter(|value| !value.trim().is_empty()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(approval)).into_response()
}

pub(crate) async fn list_logistics_exception_types(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsExceptionTypeListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let repo = LogisticsExceptionsRepo::new(state.db.pool());
    let types = match repo
        .list_exception_types(
            tenant_id,
            LogisticsExceptionTypeFilters {
                enabled: query.enabled,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(types)).into_response()
}

pub(crate) async fn create_logistics_exception_type(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsExceptionTypeCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let type_code = req.type_code.trim();
    if type_code.is_empty() {
        return bad_request("type_code required").into_response();
    }
    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }

    let repo = LogisticsExceptionsRepo::new(state.db.pool());
    let exception_type = match repo
        .upsert_exception_type(LogisticsExceptionTypeCreate {
            tenant_id,
            type_code: type_code.to_string(),
            name: name.to_string(),
            sla_hours: req.sla_hours.unwrap_or(6),
            severity: req.severity.unwrap_or_else(|| "MEDIUM".to_string()),
            enabled: req.enabled.unwrap_or(true),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(exception_type)).into_response()
}

pub(crate) async fn update_logistics_exception_type(
    State(state): State<AppState>,
    Path((tenant_id, type_code)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsExceptionTypeUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let type_code = type_code.trim();
    if type_code.is_empty() {
        return bad_request("type_code required").into_response();
    }

    let repo = LogisticsExceptionsRepo::new(state.db.pool());
    let exception_type = match repo
        .update_exception_type(
            tenant_id,
            type_code,
            LogisticsExceptionTypeUpdate {
                name: req
                    .name
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                sla_hours: req.sla_hours,
                severity: req
                    .severity
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                enabled: req.enabled,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("exception type not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(exception_type)).into_response()
}

pub(crate) async fn list_logistics_exception_work_orders(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsExceptionWorkOrderListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = LogisticsExceptionsRepo::new(state.db.pool());
    let result = match repo
        .list_work_orders(
            tenant_id,
            LogisticsExceptionWorkOrderFilters {
                status: query.status,
                exception_type: query.exception_type,
                assignee: query.assignee,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsExceptionWorkOrderListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_logistics_exception_work_order(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsExceptionWorkOrderCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let order_id = req.order_id.trim();
    if order_id.is_empty() {
        return bad_request("order_id required").into_response();
    }
    let tracking_no = req.tracking_no.trim();
    if tracking_no.is_empty() {
        return bad_request("tracking_no required").into_response();
    }
    let carrier_code = req.carrier_code.trim();
    if carrier_code.is_empty() {
        return bad_request("carrier_code required").into_response();
    }
    let exception_type = req.exception_type.trim();
    if exception_type.is_empty() {
        return bad_request("exception_type required").into_response();
    }

    let repo = LogisticsExceptionsRepo::new(state.db.pool());
    let work_order = match repo
        .create_work_order(LogisticsExceptionWorkOrderCreate {
            tenant_id,
            order_id: order_id.to_string(),
            tracking_no: tracking_no.to_string(),
            carrier_code: carrier_code.to_string(),
            exception_type: exception_type.to_string(),
            status: req.status.unwrap_or_else(|| "OPEN".to_string()),
            assignee: req.assignee,
            due_at: req.due_at,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(work_order)).into_response()
}

pub(crate) async fn assign_logistics_exception_work_order(
    State(state): State<AppState>,
    Path((tenant_id, ticket_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsExceptionWorkOrderAssignRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let ticket_id = match uuid::Uuid::parse_str(&ticket_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad ticket_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let assignee = req.assignee.trim();
    if assignee.is_empty() {
        return bad_request("assignee required").into_response();
    }

    let repo = LogisticsExceptionsRepo::new(state.db.pool());
    let work_order = match repo
        .assign_work_order(tenant_id, ticket_id, assignee.to_string())
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("work order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(work_order)).into_response()
}

pub(crate) async fn resolve_logistics_exception_work_order(
    State(state): State<AppState>,
    Path((tenant_id, ticket_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsExceptionWorkOrderResolveRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let ticket_id = match uuid::Uuid::parse_str(&ticket_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad ticket_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let resolution = req.resolution.trim();
    if resolution.is_empty() {
        return bad_request("resolution required").into_response();
    }

    let repo = LogisticsExceptionsRepo::new(state.db.pool());
    let work_order = match repo
        .resolve_work_order(tenant_id, ticket_id, resolution.to_string())
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("work order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    match OrderId::from_str(work_order.order_id.trim()) {
        Ok(order_id) => {
            if let Err(err) = refresh_profit_snapshot_for_order(
                &state,
                tenant_id,
                order_id,
                Some(ctx.user_id),
                "LOGISTICS.EXCEPTION_RESOLVED",
            )
            .await
            {
                warn!(
                    tenant_id = %tenant_id,
                    ticket_id = %ticket_id,
                    order_id = %order_id,
                    error = %err,
                    "failed to refresh profit snapshot after resolving logistics exception"
                );
            }
        }
        Err(err) => {
            warn!(
                tenant_id = %tenant_id,
                ticket_id = %ticket_id,
                order_id = %work_order.order_id,
                error = %err,
                "skip profit snapshot refresh: invalid order_id in logistics work order"
            );
        }
    }

    (StatusCode::OK, Json(work_order)).into_response()
}

pub(crate) async fn escalate_logistics_exception_work_order(
    State(state): State<AppState>,
    Path((tenant_id, ticket_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let ticket_id = match uuid::Uuid::parse_str(&ticket_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad ticket_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let repo = LogisticsExceptionsRepo::new(state.db.pool());
    let work_order = match repo.escalate_work_order(tenant_id, ticket_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("work order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(work_order)).into_response()
}

pub(crate) async fn list_logistics_warehouse_strategies(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsWarehouseStrategyListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = LogisticsWarehouseStrategiesRepo::new(state.db.pool());
    let result = match repo
        .list_strategies(
            tenant_id,
            LogisticsWarehouseStrategyListFilters {
                enabled: query.enabled,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsWarehouseStrategyListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_logistics_warehouse_strategy(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsWarehouseStrategyCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let priority = req
        .priority
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "SPEED".to_string());

    let repo = LogisticsWarehouseStrategiesRepo::new(state.db.pool());
    let strategy = match repo
        .create_strategy(LogisticsWarehouseStrategyCreate {
            tenant_id,
            name: name.to_string(),
            priority: priority.clone(),
            enabled: req.enabled.unwrap_or(true),
            updated_by: req.updated_by.clone(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let summary = req
        .history_summary
        .unwrap_or_else(|| format!("新增策略：{name}"));
    if let Err(e) = repo
        .insert_history(LogisticsWarehouseStrategyHistoryCreate {
            tenant_id,
            strategy_id: strategy.strategy_id,
            summary,
            updated_by: req.updated_by.clone(),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn update_logistics_warehouse_strategy(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsWarehouseStrategyUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let strategy_id = match uuid::Uuid::parse_str(&strategy_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad strategy_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let repo = LogisticsWarehouseStrategiesRepo::new(state.db.pool());
    let strategy = match repo
        .update_strategy(
            tenant_id,
            strategy_id,
            LogisticsWarehouseStrategyUpdate {
                name: req
                    .name
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                priority: req
                    .priority
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                enabled: req.enabled,
                updated_by: req.updated_by.clone(),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let summary = req
        .history_summary
        .unwrap_or_else(|| "更新仓配策略".to_string());
    if let Err(e) = repo
        .insert_history(LogisticsWarehouseStrategyHistoryCreate {
            tenant_id,
            strategy_id: strategy.strategy_id,
            summary,
            updated_by: req
                .updated_by
                .clone()
                .or_else(|| strategy.updated_by.clone()),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn list_logistics_warehouse_strategy_history(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let strategy_id = match uuid::Uuid::parse_str(&strategy_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad strategy_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let repo = LogisticsWarehouseStrategiesRepo::new(state.db.pool());
    let history = match repo.list_history(tenant_id, strategy_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsWarehouseStrategyHistoryListResponse { items: history };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_logistics_warehouse_suggestions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsWarehouseSuggestionListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = LogisticsWarehouseSuggestionsRepo::new(state.db.pool());
    let result = match repo
        .list_suggestions(
            tenant_id,
            LogisticsWarehouseSuggestionListFilters {
                applied: query.applied,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsWarehouseSuggestionListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn apply_logistics_warehouse_suggestion(
    State(state): State<AppState>,
    Path((tenant_id, suggestion_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let suggestion_id = match uuid::Uuid::parse_str(&suggestion_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad suggestion_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let repo = LogisticsWarehouseSuggestionsRepo::new(state.db.pool());
    let suggestion = match repo.apply_suggestion(tenant_id, suggestion_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("suggestion not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(suggestion)).into_response()
}

pub(crate) fn build_installation_task_response(
    task: LogisticsInstallationTask,
    appointments: Vec<LogisticsInstallationAppointment>,
) -> LogisticsInstallationTaskResponse {
    LogisticsInstallationTaskResponse {
        task_id: task.task_id,
        tenant_id: task.tenant_id,
        order_id: task.order_id,
        customer_name: task.customer_name,
        address: task.address,
        service_type: task.service_type,
        assigned_team: task.assigned_team,
        status: task.status,
        remedy_ticket_id: task.remedy_ticket_id,
        exception_note: task.exception_note,
        created_at: task.created_at,
        updated_at: task.updated_at,
        appointments,
    }
}

pub(crate) async fn list_logistics_installation_tasks(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsInstallationTaskListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let tasks_repo = LogisticsInstallationTasksRepo::new(state.db.pool());
    let result = match tasks_repo
        .list_tasks(
            tenant_id,
            LogisticsInstallationTaskListFilters {
                status: query.status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let task_ids: Vec<uuid::Uuid> = result.items.iter().map(|task| task.task_id).collect();
    let appointments_repo = LogisticsInstallationAppointmentsRepo::new(state.db.pool());
    let appointments = match appointments_repo
        .list_by_task_ids(tenant_id, &task_ids)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut appointment_map: HashMap<uuid::Uuid, Vec<LogisticsInstallationAppointment>> =
        HashMap::new();
    for appointment in appointments {
        appointment_map
            .entry(appointment.task_id)
            .or_default()
            .push(appointment);
    }

    let items = result
        .items
        .into_iter()
        .map(|task| {
            let appointments = appointment_map.remove(&task.task_id).unwrap_or_default();
            build_installation_task_response(task, appointments)
        })
        .collect();

    let response = LogisticsInstallationTaskListResponse {
        items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_logistics_installation_task(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsInstallationTaskCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let order_id = req.order_id.trim();
    if order_id.is_empty() {
        return bad_request("order_id required").into_response();
    }
    let customer_name = req.customer_name.trim();
    if customer_name.is_empty() {
        return bad_request("customer_name required").into_response();
    }
    let address = req.address.trim();
    if address.is_empty() {
        return bad_request("address required").into_response();
    }
    let service_type = req.service_type.trim();
    if service_type.is_empty() {
        return bad_request("service_type required").into_response();
    }

    let tasks_repo = LogisticsInstallationTasksRepo::new(state.db.pool());
    let task = match tasks_repo
        .create_task(LogisticsInstallationTaskCreate {
            tenant_id,
            order_id: order_id.to_string(),
            customer_name: customer_name.to_string(),
            address: address.to_string(),
            service_type: service_type.to_string(),
            assigned_team: req.assigned_team,
            status: "SCHEDULED".to_string(),
            remedy_ticket_id: None,
            exception_note: None,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let appointments_repo = LogisticsInstallationAppointmentsRepo::new(state.db.pool());
    if let Some(scheduled_at) = req.scheduled_at {
        if let Err(e) = appointments_repo
            .create_appointment(LogisticsInstallationAppointmentCreate {
                task_id: task.task_id,
                scheduled_at,
                status: "BOOKED".to_string(),
            })
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }

    let appointments = match appointments_repo
        .list_by_task(tenant_id, task.task_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = build_installation_task_response(task, appointments);
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn update_logistics_installation_task(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsInstallationTaskUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let task_id = match uuid::Uuid::parse_str(&task_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad task_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let tasks_repo = LogisticsInstallationTasksRepo::new(state.db.pool());
    let task = match tasks_repo
        .update_task(
            tenant_id,
            task_id,
            LogisticsInstallationTaskUpdate {
                customer_name: None,
                address: None,
                service_type: None,
                assigned_team: req.assigned_team,
                status: req.status,
                remedy_ticket_id: req.remedy_ticket_id,
                exception_note: req.exception_note,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("installation task not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let appointments_repo = LogisticsInstallationAppointmentsRepo::new(state.db.pool());
    let appointments = match appointments_repo
        .list_by_task(tenant_id, task.task_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let response = build_installation_task_response(task, appointments);
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_logistics_installation_appointment(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsInstallationAppointmentCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let task_id = match uuid::Uuid::parse_str(&task_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad task_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let tasks_repo = LogisticsInstallationTasksRepo::new(state.db.pool());
    let task = match tasks_repo.get_task(tenant_id, task_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("installation task not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let appointments_repo = LogisticsInstallationAppointmentsRepo::new(state.db.pool());
    let appointment = match appointments_repo
        .create_appointment(LogisticsInstallationAppointmentCreate {
            task_id: task.task_id,
            scheduled_at: req.scheduled_at,
            status: req.status.unwrap_or_else(|| "BOOKED".to_string()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let _ = tasks_repo
        .update_task(
            tenant_id,
            task.task_id,
            LogisticsInstallationTaskUpdate {
                customer_name: None,
                address: None,
                service_type: None,
                assigned_team: None,
                status: Some("SCHEDULED".to_string()),
                remedy_ticket_id: None,
                exception_note: None,
            },
        )
        .await;

    (StatusCode::OK, Json(appointment)).into_response()
}

pub(crate) async fn get_logistics_settlement_rule(
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
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let repo = LogisticsSettlementRulesRepo::new(state.db.pool());
    let rule = match repo.get_rule(tenant_id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            let default_rule = LogisticsSettlementRuleUpsert {
                tenant_id,
                weight_rounding: "鎸?.5KG杩涗綅".to_string(),
                min_fee_cent: 0,
                oversize_fee_cent: 0,
                overtime_fee_cent: 0,
            };
            match repo.upsert_rule(default_rule).await {
                Ok(v) => v,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                }
            }
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn update_logistics_settlement_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsSettlementRuleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let repo = LogisticsSettlementRulesRepo::new(state.db.pool());
    let rule = match repo
        .upsert_rule(LogisticsSettlementRuleUpsert {
            tenant_id,
            weight_rounding: req.weight_rounding,
            min_fee_cent: req.min_fee_cent,
            oversize_fee_cent: req.oversize_fee_cent,
            overtime_fee_cent: req.overtime_fee_cent,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn list_logistics_settlement_batches(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsSettlementBatchListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = LogisticsSettlementBatchesRepo::new(state.db.pool());
    let result = match repo
        .list_batches(
            tenant_id,
            LogisticsSettlementBatchListFilters {
                status: query.status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsSettlementBatchListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn confirm_logistics_settlement_batch(
    State(state): State<AppState>,
    Path((tenant_id, batch_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsSettlementBatchConfirmRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let batch_id = match uuid::Uuid::parse_str(&batch_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad batch_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let payment_plan_id = req
        .payment_plan_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("PAY-{}", Utc::now().timestamp()));

    let repo = LogisticsSettlementBatchesRepo::new(state.db.pool());
    let batch = match repo
        .confirm_batch(tenant_id, batch_id, Some(payment_plan_id))
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("settlement batch not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(batch)).into_response()
}

pub(crate) async fn request_shipment(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    headers: HeaderMap,
    Json(req): Json<ShipmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };

    let order_id = match OrderId::from_str(&req.order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };

    let idempotency_key = match headers.get("Idempotency-Key").and_then(|v| v.to_str().ok()) {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return bad_request("missing Idempotency-Key").into_response(),
    };

    let shipment = match handle_shipment_request(
        &state,
        &ctx,
        &req_ctx,
        ShipmentRequestArgs {
            tenant_id,
            order_id,
            warehouse_id,
            carrier_code: req.carrier_code.clone(),
            tracking_no: req.tracking_no.clone(),
            weight_gram: req.weight_gram,
            serials: req.serials.unwrap_or_default(),
            idempotency_key,
        },
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    (StatusCode::OK, Json(shipment)).into_response()
}

pub(crate) async fn preview_shipment_preprocess(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShipmentPreprocessPreviewRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
        return resp.into_response();
    }

    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => v,
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => match state.warehouses_repo.get_default_warehouse(tenant_id).await {
            Ok(Some(warehouse)) => warehouse.warehouse_id,
            Ok(None) => return bad_request("warehouse_id required").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        },
    };

    let carrier_code = match req.carrier_code.and_then(non_empty) {
        Some(value) => value,
        None => {
            let shops_repo = ShopsRepo::new(state.db.pool());
            match shops_repo
                .get_default_carrier_code(tenant_id, detail.order.shop_id)
                .await
            {
                Ok(Some(code)) => code,
                Ok(None) => return bad_request("carrier_code required").into_response(),
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                }
            }
        }
    };

    let preprocess_input = ShipmentPreprocessInput {
        warehouse_id,
        carrier_code: carrier_code.clone(),
        tracking_no: req.tracking_no.clone(),
        weight_gram: req.weight_gram,
    };

    let preprocess_match = match evaluate_shipment_preprocess(
        &state,
        tenant_id,
        &detail,
        &preprocess_input,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(tenant_id = %tenant_id, order_id = %order_id, error = %e, "shipment preprocess failed");
            None
        }
    };

    let mut result = ShipmentPreprocessPreviewResult {
        warehouse_id: Some(preprocess_input.warehouse_id),
        carrier_code: Some(preprocess_input.carrier_code.clone()),
        tracking_no: preprocess_input.tracking_no.clone(),
        weight_gram: preprocess_input.weight_gram,
        sender_address_id: None,
        auto_freeze: false,
        auto_waybill_reserve: false,
    };

    let mut matched = false;
    let mut strategy_id = None;
    let mut hit_reason = None;

    if let Some(match_result) = preprocess_match.as_ref() {
        matched = true;
        strategy_id = Some(match_result.strategy.strategy_id);
        hit_reason = Some(match_result.hit_reason.clone());
        if let Some(warehouse_id) = match_result.rules.warehouse_id {
            result.warehouse_id = Some(warehouse_id);
        }
        if let Some(carrier_code) = match_result.rules.carrier_code.as_ref() {
            result.carrier_code = Some(carrier_code.clone());
        }
        if let Some(tracking_no) = match_result.rules.tracking_no.as_ref() {
            result.tracking_no = Some(tracking_no.clone());
        }
        if let Some(weight_gram) = match_result.rules.weight_gram {
            result.weight_gram = Some(weight_gram);
        }
        result.sender_address_id = match_result.strategy.sender_address_id;
        result.auto_freeze = match_result.strategy.auto_freeze;
        result.auto_waybill_reserve = match_result.strategy.auto_waybill_reserve;
    }

    if let Some(match_result) = preprocess_match.as_ref() {
        let hits_repo = ShipmentPreprocessHitsRepo::new(state.db.pool());
        let result_json = serde_json::json!({
            "warehouse_id": result.warehouse_id,
            "carrier_code": result.carrier_code,
            "tracking_no": result.tracking_no,
            "weight_gram": result.weight_gram,
            "sender_address_id": result.sender_address_id,
            "auto_freeze": result.auto_freeze,
            "auto_waybill_reserve": result.auto_waybill_reserve,
            "trigger": "PREVIEW"
        });
        if let Err(e) = hits_repo
            .insert_hit(ShipmentPreprocessHitCreate {
                tenant_id,
                strategy_id: Some(match_result.strategy.strategy_id),
                shipment_id: None,
                order_id: Some(uuid::Uuid::from(order_id)),
                hit_reason: Some(match_result.hit_reason.clone()),
                result_json: Some(result_json),
                success: true,
            })
            .await
        {
            warn!(error = %e, "shipment preprocess hit insert failed");
        }
    }

    (
        StatusCode::OK,
        Json(ShipmentPreprocessPreviewResponse {
            matched,
            strategy_id,
            hit_reason,
            result,
        }),
    )
        .into_response()
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ShipmentPreprocessRules {
    warehouse_id: Option<uuid::Uuid>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    weight_gram: Option<i32>,
}

struct ShipmentPreprocessMatch {
    strategy: ShipmentPreprocessStrategy,
    hit_reason: String,
    rules: ShipmentPreprocessRules,
}

struct ShipmentPreprocessInput {
    warehouse_id: uuid::Uuid,
    carrier_code: String,
    tracking_no: Option<String>,
    weight_gram: Option<i32>,
}

struct LogisticsMatchInput {
    warehouse_id: Option<uuid::Uuid>,
    weight_gram: Option<i32>,
}

pub(crate) async fn handle_shipment_request(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    args: ShipmentRequestArgs,
) -> Result<ShipmentView, axum::response::Response> {
    let ShipmentRequestArgs {
        tenant_id,
        order_id,
        warehouse_id,
        carrier_code,
        tracking_no,
        weight_gram,
        serials,
        idempotency_key,
    } = args;
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return Err(resp.into_response());
    }
    if let Err(resp) = ensure_permission(ctx, "orders.write") {
        return Err(resp.into_response());
    }
    if let Err(resp) = ensure_permission(ctx, "shipping.write") {
        return Err(resp.into_response());
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let Some(order) = (match orders_repo
        .get_order_shipping_info(tenant_id, order_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()),
    }) else {
        return Err(not_found("order not found").into_response());
    };
    if let Err(resp) = ensure_shop_access(ctx, order.shop_id) {
        return Err(resp.into_response());
    }

    let shipping_enabled = match feature_flag_enabled(
        state,
        tenant_id,
        Some(order.shop_id),
        FeatureFlagKey::ShippingWrite,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return Err(resp.into_response()),
    };
    if !shipping_enabled {
        warn!(
            tenant_id = %tenant_id,
            shop_id = %order.shop_id,
            order_id = %order_id,
            "feature disabled: shipping write"
        );
        return Err(feature_flag_forbidden(FeatureFlagKey::ShippingWrite).into_response());
    }

    let pdd_only = match pdd_only_mode_enabled(state, tenant_id).await {
        Ok(v) => v,
        Err(resp) => return Err(resp.into_response()),
    };
    if pdd_only && order.platform != Platform::Pdd {
        warn!(
            tenant_id = %tenant_id,
            order_id = %order_id,
            platform = %order.platform,
            "pdd-only mode blocked shipment request"
        );
        return Err(pdd_only_forbidden().into_response());
    }

    if order.status != OrderStatus::PendingShipment && order.status != OrderStatus::Allocated {
        return Err(conflict("order status not shippable").into_response());
    }

    let warehouse_id = match warehouse_id {
        Some(id) => id,
        None => match state.warehouses_repo.get_default_warehouse(tenant_id).await {
            Ok(Some(warehouse)) => warehouse.warehouse_id,
            Ok(None) => return Err(bad_request("warehouse_id required").into_response()),
            Err(e) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())
            }
        },
    };
    if let Err(resp) = ensure_warehouse_access(ctx, warehouse_id) {
        return Err(resp.into_response());
    }

    let carrier_code = match carrier_code.and_then(non_empty) {
        Some(value) => Some(value),
        None => {
            let shops_repo = ShopsRepo::new(state.db.pool());
            match shops_repo
                .get_default_carrier_code(tenant_id, order.shop_id)
                .await
            {
                Ok(Some(code)) => Some(code),
                Ok(None) => None,
                Err(e) => {
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())
                }
            }
        }
    };

    let carrier_code = carrier_code.and_then(|code| {
        let trimmed = code.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let shipment_id = uuid::Uuid::new_v5(&uuid::Uuid::from(tenant_id), idempotency_key.as_bytes());
    let requested_at = Utc::now();
    let order_detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(detail)) => detail,
        Ok(None) => return Err(not_found("order not found").into_response()),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()),
    };

    let logistics_match_input = LogisticsMatchInput {
        warehouse_id: Some(warehouse_id),
        weight_gram,
    };
    let logistics_match = match evaluate_logistics_match(
        state,
        tenant_id,
        &order_detail,
        &logistics_match_input,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(tenant_id = %tenant_id, order_id = %order_id, error = %e, "logistics match failed");
            None
        }
    };

    let resolved_carrier_code = carrier_code.or_else(|| {
        logistics_match.as_ref().and_then(|matched| {
            matched
                .product
                .as_ref()
                .map(|product| product.carrier_code.clone())
        })
    });
    let mut resolved_carrier_code = match resolved_carrier_code {
        Some(code) => code,
        None => return Err(bad_request("carrier_code required").into_response()),
    };

    let preprocess_input = ShipmentPreprocessInput {
        warehouse_id,
        carrier_code: resolved_carrier_code.clone(),
        tracking_no: tracking_no.clone(),
        weight_gram,
    };
    let preprocess_match = match evaluate_shipment_preprocess(
        state,
        tenant_id,
        &order_detail,
        &preprocess_input,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(tenant_id = %tenant_id, order_id = %order_id, error = %e, "shipment preprocess failed");
            None
        }
    };

    let mut resolved_warehouse_id = warehouse_id;
    let mut resolved_tracking_no = tracking_no.clone();
    let mut resolved_weight_gram = weight_gram;
    let mut sender_address_id: Option<uuid::Uuid> = None;
    let mut auto_freeze = false;
    let mut auto_waybill_reserve = true;

    if let Some(match_result) = preprocess_match.as_ref() {
        if let Some(next_warehouse) = match_result.rules.warehouse_id {
            resolved_warehouse_id = next_warehouse;
        }
        if let Some(next_carrier) = match_result.rules.carrier_code.as_ref() {
            resolved_carrier_code = next_carrier.clone();
        }
        if let Some(next_tracking) = match_result.rules.tracking_no.as_ref() {
            resolved_tracking_no = Some(next_tracking.clone());
        }
        if let Some(next_weight) = match_result.rules.weight_gram {
            resolved_weight_gram = Some(next_weight);
        }
        sender_address_id = match_result.strategy.sender_address_id;
        auto_freeze = match_result.strategy.auto_freeze;
        auto_waybill_reserve = match_result.strategy.auto_waybill_reserve;
    }

    if let Some(match_result) = logistics_match.as_ref() {
        let hits_repo = LogisticsMatchHitsRepo::new(state.db.pool());
        let result_json = serde_json::json!({
            "rule_name": match_result.rule.name,
            "product_id": match_result.product.as_ref().map(|product| product.product_id),
            "carrier_code": match_result.product.as_ref().map(|product| product.carrier_code.clone()),
            "warehouse_id": resolved_warehouse_id,
            "weight_gram": resolved_weight_gram,
            "trigger": "SHIPMENT_REQUEST"
        });
        if let Err(err) = hits_repo
            .insert_hit(LogisticsMatchHitCreate {
                tenant_id,
                rule_id: Some(match_result.rule.rule_id),
                shipment_id: Some(shipment_id),
                order_id: Some(uuid::Uuid::from(order_id)),
                product_id: match_result
                    .product
                    .as_ref()
                    .map(|product| product.product_id),
                hit_reason: Some(match_result.rule.name.clone()),
                result_json: Some(result_json),
                success: true,
            })
            .await
        {
            warn!(error = %err, "logistics match hit insert failed");
        }
    }

    let waybill_enabled = match feature_flag_enabled(
        state,
        tenant_id,
        None,
        FeatureFlagKey::WaybillAutoReserve,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return Err(resp.into_response()),
    };

    let mut auto_waybill_reserved = false;
    if resolved_tracking_no.is_none() {
        if auto_waybill_reserve && waybill_enabled {
            let waybills_repo = WaybillsRepo::new(state.db.pool());
            let reserved_waybill = match waybills_repo
                .reserve_waybill(
                    tenant_id,
                    uuid::Uuid::from(order.shop_id),
                    &resolved_carrier_code,
                    shipment_id,
                    None,
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
            resolved_tracking_no = Some(reserved_waybill.waybill_no.clone());
            auto_waybill_reserved = true;
        } else {
            return Err(bad_request("tracking_no required").into_response());
        }
    }

    let shipments_repo = ShipmentsRepo::new(state.db.pool());

    let (shipment, created) = match shipments_repo
        .create_requested(ShipmentCreateInput {
            shipment_id,
            tenant_id,
            shipment_type: SHIPMENT_TYPE_PLATFORM.to_string(),
            shop_id: uuid::Uuid::from(order.shop_id),
            order_id: uuid::Uuid::from(order_id),
            platform: order.platform.to_string(),
            platform_order_id: order.platform_order_id.clone(),
            warehouse_id: resolved_warehouse_id,
            shipment_no: Some(shipment_id.to_string()),
            carrier_code: Some(resolved_carrier_code.clone()),
            tracking_no: resolved_tracking_no.clone(),
            requested_at,
            weight_gram: resolved_weight_gram,
            sender_address_id,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => {
            if e.to_string().contains("idempotency key conflict") {
                return Err(conflict("idempotency key conflict").into_response());
            }
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
        }
    };

    if created {
        let mut auto_freeze_applied = false;
        if auto_freeze {
            let auto_freeze_enabled = match feature_flag_enabled(
                state,
                tenant_id,
                None,
                FeatureFlagKey::AutoFreeze,
            )
            .await
            {
                Ok(v) => v,
                Err(resp) => return Err(resp.into_response()),
            };
            if auto_freeze_enabled {
                let item_lines = match orders_repo.list_order_items(tenant_id, order_id).await {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(
                            tenant_id = %tenant_id,
                            order_id = %order_id,
                            error = %e,
                            "shipment preprocess auto freeze failed"
                        );
                        Vec::new()
                    }
                };
                let mut sku_qty: HashMap<uuid::Uuid, i64> = HashMap::new();
                for line in item_lines {
                    if let Some(raw) = line.sku_id.as_deref() {
                        if let Ok(sku_id) = uuid::Uuid::parse_str(raw) {
                            *sku_qty.entry(sku_id).or_insert(0) += line.qty;
                        }
                    }
                }
                if !sku_qty.is_empty() {
                    let freeze_repo = InventoryFreezesRepo::new(state.db.pool());
                    for (sku_id, qty) in sku_qty {
                        let _ = freeze_repo
                            .create_freeze(InventoryFreezeCreateInput {
                                tenant_id,
                                warehouse_id: resolved_warehouse_id,
                                sku_id,
                                qty,
                                reason: Some(format!(
                                    "SHIPMENT_PREPROCESS:{}",
                                    shipment.shipment_id
                                )),
                            })
                            .await;
                    }
                    auto_freeze_applied = true;
                }
            }
        }

        if let Some(match_result) = preprocess_match.as_ref() {
            let hits_repo = ShipmentPreprocessHitsRepo::new(state.db.pool());
            let result_json = serde_json::json!({
                "warehouse_id": resolved_warehouse_id,
                "carrier_code": resolved_carrier_code,
                "tracking_no": resolved_tracking_no,
                "weight_gram": resolved_weight_gram,
                "sender_address_id": sender_address_id,
                "auto_freeze": auto_freeze,
                "auto_freeze_applied": auto_freeze_applied,
                "auto_waybill_reserve": auto_waybill_reserve,
                "auto_waybill_reserved": auto_waybill_reserved,
                "trigger": "SHIPMENT_CREATED"
            });
            if let Err(e) = hits_repo
                .insert_hit(ShipmentPreprocessHitCreate {
                    tenant_id,
                    strategy_id: Some(match_result.strategy.strategy_id),
                    shipment_id: Some(shipment.shipment_id),
                    order_id: Some(uuid::Uuid::from(order_id)),
                    hit_reason: Some(match_result.hit_reason.clone()),
                    result_json: Some(result_json),
                    success: true,
                })
                .await
            {
                warn!(error = %e, "shipment preprocess hit insert failed");
            }
        }

        let payload = ShipmentRequestedPayload {
            tenant_id,
            shipment_id: shipment.shipment_id,
            order_id,
            shop_id: order.shop_id,
            platform: order.platform,
            platform_order_id: order.platform_order_id,
            warehouse_id,
            carrier_code: shipment.carrier_code.clone(),
            tracking_no: shipment.tracking_no.clone(),
            weight_gram: shipment.weight_gram,
        };
        let outbox_repo = OutboxRepo::new(state.db.pool());
        let out_payload = match serde_json::to_value(payload) {
            Ok(v) => v,
            Err(e) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())
            }
        };
        if let Err(e) = outbox_repo
            .enqueue_with_trace(
                tenant_id,
                SHIPMENT_REQUESTED_EVENT,
                &out_payload,
                Some(req_ctx.trace_id.as_str()),
            )
            .await
        {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
        }

        let log_metadata = serde_json::json!({
            "shipment_id": shipment.shipment_id,
            "order_id": order_id.to_string(),
            "warehouse_id": resolved_warehouse_id,
            "carrier_code": shipment.carrier_code.clone(),
            "tracking_no": shipment.tracking_no.clone(),
        });
        let logs_repo = OperationLogsRepo::new(state.db.pool());
        if let Err(e) = logs_repo
            .insert(OperationLogCreate {
                tenant_id,
                actor_type: "USER".to_string(),
                actor_user_id: Some(ctx.user_id),
                action: OP_ACTION_SHIPMENT_REQUESTED.to_string(),
                ref_type: Some("SHIPMENT".to_string()),
                ref_id: Some(shipment.shipment_id.to_string()),
                metadata: Some(log_metadata),
            })
            .await
        {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
        }

        record_audit_log(
            state,
            ctx,
            req_ctx,
            "OMS.SHIPPING.REQUEST",
            "shipment",
            Some(shipment.shipment_id.to_string()),
            None,
            Some(serde_json::json!({
                "shipment_id": shipment.shipment_id,
                "order_id": order_id,
                "shop_id": order.shop_id,
                "status": shipment.status,
                "warehouse_id": shipment.warehouse_id,
            })),
        )
        .await;
    }

    if !serials.is_empty() {
        let bound = match bind_serials_for_shipment(
            state,
            tenant_id,
            shipment.shipment_id,
            order_id,
            serials,
        )
        .await
        {
            Ok(v) => v,
            Err(resp) => return Err(resp),
        };
        record_audit_log(
            state,
            ctx,
            req_ctx,
            "SHIPMENT.SERIAL_BIND",
            "shipment",
            Some(shipment.shipment_id.to_string()),
            None,
            Some(serde_json::json!({
                "shipment_id": shipment.shipment_id,
                "bound_count": bound.len(),
            })),
        )
        .await;
    }

    let display_warehouse_id = shipment.warehouse_id.unwrap_or(resolved_warehouse_id);
    let display_carrier_code = shipment
        .carrier_code
        .clone()
        .or(Some(resolved_carrier_code.clone()))
        .unwrap_or_default();
    let created_at = shipment.requested_at.unwrap_or(shipment.updated_at);

    Ok(ShipmentView {
        shipment_id: shipment.shipment_id,
        tenant_id: uuid::Uuid::from(shipment.tenant_id),
        order_id: shipment.order_id,
        shop_id: shipment.shop_id,
        platform: shipment.platform,
        warehouse_id: display_warehouse_id,
        carrier_code: display_carrier_code,
        tracking_no: shipment.tracking_no,
        status: shipment.status,
        ship_status: shipment.ship_status,
        weight_gram: shipment.weight_gram,
        sender_address_id: shipment.sender_address_id,
        last_push_error: shipment.last_push_error,
        push_status: None,
        push_error_message: None,
        push_attempted_at: None,
        printed_label: false,
        created_at,
        updated_at: shipment.updated_at,
    })
}

pub(crate) async fn list_shipments(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ShipmentListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    if let Some(shop_id) = shop_id {
        if let Err(resp) = ensure_shop_access(&ctx, ShopId(shop_id)) {
            return resp.into_response();
        }
    }
    let updated_from = match query.updated_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad updated_from").into_response(),
        },
        None => None,
    };
    let updated_to = match query.updated_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad updated_to").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = ShipmentsRepo::new(state.db.pool());
    let result = match repo
        .list_shipments(
            tenant_id,
            ShipmentListFilters {
                shop_id,
                shop_ids: shop_scope_filter(&ctx)
                    .map(|ids| ids.into_iter().map(uuid::Uuid::from).collect()),
                status: query.status.and_then(non_empty),
                platform: query.platform.and_then(non_empty),
                shipment_type: Some(SHIPMENT_TYPE_PLATFORM.to_string()),
                tracking_no: query.tracking_no.and_then(non_empty),
                updated_from,
                updated_to,
                limit,
                offset,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shipment_ids: Vec<uuid::Uuid> = result.items.iter().map(|item| item.shipment_id).collect();
    let push_map = match load_latest_push_attempts(state.db.pool(), tenant_id, &shipment_ids).await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let data = result
        .items
        .into_iter()
        .map(|shipment| {
            let created_at = shipment.requested_at.unwrap_or(shipment.updated_at);
            let push_summary = push_map.get(&shipment.shipment_id);
            ShipmentView {
                shipment_id: shipment.shipment_id,
                tenant_id: uuid::Uuid::from(shipment.tenant_id),
                order_id: shipment.order_id,
                shop_id: shipment.shop_id,
                platform: shipment.platform,
                warehouse_id: shipment.warehouse_id.unwrap_or_default(),
                carrier_code: shipment.carrier_code.unwrap_or_default(),
                tracking_no: shipment.tracking_no,
                status: shipment.status,
                ship_status: shipment.ship_status,
                weight_gram: shipment.weight_gram,
                sender_address_id: shipment.sender_address_id,
                last_push_error: shipment.last_push_error,
                push_status: push_summary.map(|summary| summary.status.clone()),
                push_error_message: push_summary.map(|summary| summary.error_message.clone()),
                push_attempted_at: push_summary.map(|summary| summary.attempted_at),
                printed_label: false,
                created_at,
                updated_at: shipment.updated_at,
            }
        })
        .collect();

    let response = ShipmentListResponse { data };
    (StatusCode::OK, Json(response)).into_response()
}

async fn evaluate_shipment_preprocess(
    state: &AppState,
    tenant_id: TenantId,
    detail: &erp_repo::orders::OmsOrderDetail,
    input: &ShipmentPreprocessInput,
) -> anyhow::Result<Option<ShipmentPreprocessMatch>> {
    let repo = ShipmentPreprocessStrategiesRepo::new(state.db.pool());
    let strategies = repo.list_enabled_strategies(tenant_id).await?;
    if strategies.is_empty() {
        return Ok(None);
    }

    let now = Utc::now();
    let context = build_shipment_preprocess_context(detail, input);

    for strategy in strategies {
        if let Some(from) = strategy.effective_from {
            if now < from {
                continue;
            }
        }
        if let Some(to) = strategy.effective_to {
            if now > to {
                continue;
            }
        }

        let conditions: Vec<OrderReviewCondition> =
            serde_json::from_value(strategy.conditions_json.clone()).unwrap_or_default();
        let matched = if conditions.is_empty() {
            false
        } else {
            evaluate_conditions(&context, &conditions)
        };

        if matched {
            let rules: ShipmentPreprocessRules =
                serde_json::from_value(strategy.rules_json.clone()).unwrap_or_default();
            let hit_reason = format!("strategy {} matched", strategy.name);
            return Ok(Some(ShipmentPreprocessMatch {
                strategy,
                hit_reason,
                rules,
            }));
        }
    }

    Ok(None)
}

fn build_shipment_preprocess_context(
    detail: &erp_repo::orders::OmsOrderDetail,
    input: &ShipmentPreprocessInput,
) -> OrderReviewContext {
    let mut context = OrderReviewContext::new();
    context.insert_field(
        "total_cent",
        serde_json::Value::Number(detail.order.total_amount_cent.into()),
    );
    context.insert_field(
        "sku_count",
        serde_json::Value::Number(
            detail
                .items
                .iter()
                .map(|item| item.quantity as i64)
                .sum::<i64>()
                .into(),
        ),
    );
    context.insert_field(
        "sku_lines",
        serde_json::Value::Number((detail.items.len() as i64).into()),
    );
    context.insert_field(
        "platform",
        serde_json::Value::String(detail.order.platform.to_string()),
    );
    context.insert_field(
        "shop_id",
        serde_json::Value::String(detail.order.shop_id.to_string()),
    );
    context.insert_field(
        "warehouse_id",
        serde_json::Value::String(input.warehouse_id.to_string()),
    );
    context.insert_field(
        "carrier_code",
        serde_json::Value::String(input.carrier_code.clone()),
    );
    if let Some(tracking_no) = input.tracking_no.as_ref() {
        context.insert_field(
            "tracking_no",
            serde_json::Value::String(tracking_no.clone()),
        );
    }
    if let Some(weight_gram) = input.weight_gram {
        context.insert_field("weight_gram", serde_json::Value::Number(weight_gram.into()));
    }
    if let Some(phone) = detail.order.receiver_phone.as_ref() {
        context.insert_field("receiver_phone", serde_json::Value::String(phone.clone()));
    }
    if let Some(name) = detail.order.receiver_name.as_ref() {
        context.insert_field("receiver_name", serde_json::Value::String(name.clone()));
    }
    if let Some(address) = detail.order.receiver_address.as_ref() {
        if let Some(province) = extract_province(address) {
            context.insert_field("address_province", serde_json::Value::String(province));
        }
        if address.trim().is_empty() {
            context.insert_field("address_invalid", serde_json::Value::Bool(true));
        }
    } else {
        context.insert_field("address_invalid", serde_json::Value::Bool(true));
    }

    context
}

struct LogisticsMatchEvaluation {
    rule: LogisticsMatchRule,
    product: Option<LogisticsProduct>,
}

async fn evaluate_logistics_match(
    state: &AppState,
    tenant_id: TenantId,
    detail: &erp_repo::orders::OmsOrderDetail,
    input: &LogisticsMatchInput,
) -> anyhow::Result<Option<LogisticsMatchEvaluation>> {
    let repo = LogisticsMatchRulesRepo::new(state.db.pool());
    let rules = repo.list_enabled_rules(tenant_id).await?;
    if rules.is_empty() {
        return Ok(None);
    }

    let context = build_logistics_match_context(detail, input);
    let products_repo = LogisticsProductsRepo::new(state.db.pool());

    for rule in rules {
        let conditions: Vec<OrderReviewCondition> =
            serde_json::from_value(rule.conditions_json.clone()).unwrap_or_default();
        let matched = if conditions.is_empty() {
            false
        } else {
            evaluate_conditions(&context, &conditions)
        };
        if matched {
            let product = match rule.product_id {
                Some(product_id) => products_repo.get_product(tenant_id, product_id).await?,
                None => None,
            };
            return Ok(Some(LogisticsMatchEvaluation { rule, product }));
        }
    }

    Ok(None)
}

fn build_logistics_match_context(
    detail: &erp_repo::orders::OmsOrderDetail,
    input: &LogisticsMatchInput,
) -> OrderReviewContext {
    let mut context = OrderReviewContext::new();
    context.insert_field(
        "total_cent",
        serde_json::Value::Number(detail.order.total_amount_cent.into()),
    );
    context.insert_field(
        "sku_count",
        serde_json::Value::Number(
            detail
                .items
                .iter()
                .map(|item| item.quantity as i64)
                .sum::<i64>()
                .into(),
        ),
    );
    context.insert_field(
        "sku_lines",
        serde_json::Value::Number((detail.items.len() as i64).into()),
    );
    context.insert_field(
        "platform",
        serde_json::Value::String(detail.order.platform.to_string()),
    );
    context.insert_field(
        "shop_id",
        serde_json::Value::String(detail.order.shop_id.to_string()),
    );
    let sku_ids: Vec<Value> = detail
        .items
        .iter()
        .filter_map(|item| {
            item.outer_sku_id
                .clone()
                .or_else(|| item.platform_sku_id.clone())
                .map(Value::String)
        })
        .collect();
    if !sku_ids.is_empty() {
        context.insert_field("sku_ids", Value::Array(sku_ids));
    }
    if let Some(warehouse_id) = input.warehouse_id {
        context.insert_field("warehouse_id", Value::String(warehouse_id.to_string()));
    }
    if let Some(weight_gram) = input.weight_gram {
        context.insert_field("weight_gram", Value::Number(weight_gram.into()));
    }
    if let Some(phone) = detail.order.receiver_phone.as_ref() {
        context.insert_field("receiver_phone", Value::String(phone.clone()));
    }
    if let Some(name) = detail.order.receiver_name.as_ref() {
        context.insert_field("receiver_name", Value::String(name.clone()));
    }
    if let Some(address) = detail.order.receiver_address.as_ref() {
        if let Some(province) = extract_province(address) {
            context.insert_field("address_province", Value::String(province));
        }
        if address.trim().is_empty() {
            context.insert_field("address_invalid", Value::Bool(true));
        }
    } else {
        context.insert_field("address_invalid", Value::Bool(true));
    }

    context
}

pub(crate) fn extract_province(address: &str) -> Option<String> {
    const PROVINCES: [&str; 34] = [
        "鍖椾含",
        "澶╂触",
        "娌冲寳",
        "灞辫タ",
        "内蒙古",
        "杈藉畞",
        "鍚夋灄",
        "黑龙江",
        "涓婃捣",
        "姹熻嫃",
        "娴欐睙",
        "瀹夊窘",
        "绂忓缓",
        "姹熻タ",
        "灞变笢",
        "娌冲崡",
        "婀栧寳",
        "婀栧崡",
        "骞夸笢",
        "骞胯タ",
        "娴峰崡",
        "閲嶅簡",
        "鍥涘窛",
        "璐靛窞",
        "浜戝崡",
        "瑗胯棌",
        "闄曡タ",
        "鐢樿們",
        "闈掓捣",
        "瀹佸",
        "鏂扮枂",
        "棣欐腐",
        "婢抽棬",
        "鍙版咕",
    ];
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return None;
    }
    for province in PROVINCES {
        if trimmed.contains(province) {
            return Some(province.to_string());
        }
    }
    None
}

pub(crate) fn normalize_waybill_status(raw: &str) -> Option<String> {
    let status = raw.trim().to_uppercase();
    match status.as_str() {
        WAYBILL_STATUS_AVAILABLE
        | WAYBILL_STATUS_RESERVED
        | WAYBILL_STATUS_USED
        | WAYBILL_STATUS_CANCELLED => Some(status),
        _ => None,
    }
}

pub(crate) fn waybill_view(waybill: Waybill) -> WaybillView {
    WaybillView {
        waybill_id: waybill.waybill_id,
        shop_id: waybill.shop_id,
        carrier_code: waybill.carrier_code,
        waybill_no: waybill.waybill_no,
        status: waybill.status,
        reserved_at: waybill.reserved_at,
        used_at: waybill.used_at,
        shipment_id: waybill.shipment_id,
    }
}

pub(crate) async fn import_waybills(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WaybillImportQuery>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json");
    let is_text = content_type.contains("text/csv")
        || content_type.contains("text/plain")
        || content_type.contains("application/csv");

    let (shop_id_raw, carrier_code_raw, waybills) = if is_text {
        let Some(shop_id) = query.shop_id.as_deref() else {
            return bad_request("missing shop_id").into_response();
        };
        let Some(carrier_code) = query.carrier_code.as_deref() else {
            return bad_request("missing carrier_code").into_response();
        };
        let raw = String::from_utf8_lossy(&body);
        let waybills = match parse_waybill_text(&raw) {
            Ok(v) => v,
            Err(err) => return bad_request(&err.to_string()).into_response(),
        };
        (shop_id.to_string(), carrier_code.to_string(), waybills)
    } else {
        let req: WaybillImportRequest = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(_) => return bad_request("invalid request body").into_response(),
        };
        let waybills = if let Some(list) = req.waybills {
            list
        } else if let Some(content) = req.content {
            match parse_waybill_text(&content) {
                Ok(v) => v,
                Err(err) => return bad_request(&err.to_string()).into_response(),
            }
        } else {
            Vec::new()
        };
        (req.shop_id, req.carrier_code, waybills)
    };

    let shop_id = match uuid::Uuid::parse_str(shop_id_raw.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, ShopId(shop_id)) {
        return resp.into_response();
    }
    let carrier_code = carrier_code_raw.trim();
    if carrier_code.is_empty() {
        return bad_request("carrier_code required").into_response();
    }
    if waybills.is_empty() {
        return bad_request("waybills required").into_response();
    }

    let repo = WaybillsRepo::new(state.db.pool());
    let result = match repo
        .import_waybills(tenant_id, shop_id, carrier_code, &waybills)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = WaybillImportResponse {
        inserted: result.inserted,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn allocate_waybill(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WaybillAllocateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }
    let waybill_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::WaybillAutoReserve)
            .await
        {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !waybill_enabled {
        return feature_flag_forbidden(FeatureFlagKey::WaybillAutoReserve).into_response();
    }

    let count = req.count.unwrap_or(1);
    if count != 1 {
        return bad_request("count must be 1").into_response();
    }

    let shipment_id = match req.shipment_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => v,
            Err(_) => return bad_request("bad shipment_id").into_response(),
        },
        None => return bad_request("shipment_id required").into_response(),
    };
    let shop_id = match req.shop_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => v,
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => return bad_request("shop_id required").into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, ShopId(shop_id)) {
        return resp.into_response();
    }
    let carrier_code = req.carrier_code.as_deref().map(str::trim).unwrap_or("");
    if carrier_code.is_empty() {
        return bad_request("carrier_code required").into_response();
    }

    let repo = WaybillsRepo::new(state.db.pool());
    let waybill = match repo
        .reserve_waybill(
            tenant_id,
            shop_id,
            carrier_code,
            shipment_id,
            req.waybill_no.as_deref(),
        )
        .await
    {
        Ok(v) => v,
        Err(err) => {
            let message = err.to_string();
            if message.contains("no available waybill")
                || message.contains("shipment already reserved")
            {
                return conflict("waybill not available").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
        }
    };

    let response = WaybillAllocateResponse {
        waybills: vec![waybill.waybill_no],
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn release_waybill(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WaybillReleaseRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let shipment_id = match uuid::Uuid::parse_str(req.shipment_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };
    let repo = WaybillsRepo::new(state.db.pool());
    let waybill = match repo.release_by_shipment(tenant_id, shipment_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("waybill not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(waybill_view(waybill))).into_response()
}

pub(crate) async fn cancel_waybill(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WaybillReleaseRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let shipment_id = match uuid::Uuid::parse_str(req.shipment_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };
    let repo = WaybillsRepo::new(state.db.pool());
    let waybill = match repo.cancel_by_shipment(tenant_id, shipment_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("waybill not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(waybill_view(waybill))).into_response()
}

pub(crate) async fn list_waybills(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WaybillListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    if let Some(shop_id) = shop_id {
        if let Err(resp) = ensure_shop_access(&ctx, ShopId(shop_id)) {
            return resp.into_response();
        }
    }
    let shipment_id = match query.shipment_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shipment_id").into_response(),
        },
        None => None,
    };
    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let _created_from = match query.created_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad created_from").into_response(),
        },
        None => None,
    };
    let _created_to = match query.created_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad created_to").into_response(),
        },
        None => None,
    };
    let status = match query.status.as_deref() {
        Some(raw) => match normalize_waybill_status(raw) {
            Some(v) => Some(v),
            None => return bad_request("bad status").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = WaybillsRepo::new(state.db.pool());
    let result = match repo
        .list_waybills(
            tenant_id,
            WaybillListFilters {
                shop_id,
                carrier_code: query.carrier_code.and_then(non_empty),
                status,
                shipment_id,
                waybill_no: query.waybill_no.and_then(non_empty),
                limit,
                offset,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result.items.into_iter().map(waybill_view).collect();
    let response = WaybillListResponse {
        items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_waybill_summary(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WaybillListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    if let Some(shop_id) = shop_id {
        if let Err(resp) = ensure_shop_access(&ctx, ShopId(shop_id)) {
            return resp.into_response();
        }
    }

    let repo = WaybillsRepo::new(state.db.pool());
    let summary = match repo
        .summarize_status(tenant_id, shop_id, query.carrier_code.and_then(non_empty))
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = WaybillSummaryResponse {
        available: summary.available,
        reserved: summary.reserved,
        used: summary.used,
        cancelled: summary.cancelled,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) fn parse_csv_datetime(raw: Option<String>) -> anyhow::Result<Option<DateTime<Utc>>> {
    let Some(value) = raw else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    let dt = DateTime::parse_from_rfc3339(value.trim())
        .map_err(|_| anyhow::anyhow!("invalid occurred_at"))?;
    Ok(Some(dt.with_timezone(&Utc)))
}

pub(crate) fn parse_csv_json(raw: Option<String>) -> anyhow::Result<Value> {
    let Some(value) = raw else {
        return Ok(serde_json::json!({}));
    };
    if value.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    Ok(serde_json::from_str(value.trim())?)
}

pub(crate) fn parse_platform_lines_csv(
    body: &[u8],
) -> anyhow::Result<Vec<PlatformStatementLineInput>> {
    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_reader(body);
    let mut lines = Vec::new();
    for record in reader.deserialize::<CsvPlatformStatementLine>() {
        let row = record?;
        lines.push(PlatformStatementLineInput {
            platform_order_id: row.platform_order_id,
            gross_cent: row.gross_cent.unwrap_or(0),
            fee_cent: row.fee_cent.unwrap_or(0),
            refund_cent: row.refund_cent.unwrap_or(0),
            net_cent: row.net_cent.unwrap_or(0),
            occurred_at: parse_csv_datetime(row.occurred_at)?,
            raw_json: parse_csv_json(row.raw_json)?,
        });
    }
    Ok(lines)
}

pub(crate) fn parse_shipping_lines_csv(
    body: &[u8],
) -> anyhow::Result<Vec<ShippingStatementLineInput>> {
    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_reader(body);
    let mut lines = Vec::new();
    for record in reader.deserialize::<CsvShippingStatementLine>() {
        let row = record?;
        lines.push(ShippingStatementLineInput {
            tracking_no: row.tracking_no,
            fee_cent: row.fee_cent.unwrap_or(0),
            weight: row.weight,
            occurred_at: parse_csv_datetime(row.occurred_at)?,
            raw_json: parse_csv_json(row.raw_json)?,
        });
    }
    Ok(lines)
}

pub(crate) fn parse_payment_lines_csv(
    body: &[u8],
) -> anyhow::Result<Vec<PaymentStatementLineInput>> {
    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_reader(body);
    let mut lines = Vec::new();
    for record in reader.deserialize::<CsvPaymentStatementLine>() {
        let row = record?;
        lines.push(PaymentStatementLineInput {
            platform_order_id: row.platform_order_id,
            paid_cent: row.paid_cent.unwrap_or(0),
            fee_cent: row.fee_cent.unwrap_or(0),
            net_cent: row.net_cent.unwrap_or(0),
            occurred_at: parse_csv_datetime(row.occurred_at)?,
            raw_json: parse_csv_json(row.raw_json)?,
        });
    }
    Ok(lines)
}

pub(crate) fn split_waybill_suffix(value: &str) -> Option<(String, u64, usize)> {
    let mut split = value.len();
    for (idx, ch) in value.char_indices().rev() {
        if ch.is_ascii_digit() {
            split = idx;
        } else {
            break;
        }
    }
    if split == value.len() {
        return None;
    }
    let (prefix, digits) = value.split_at(split);
    let number = digits.parse::<u64>().ok()?;
    Some((prefix.to_string(), number, digits.len()))
}

pub(crate) fn expand_waybill_token(token: &str) -> anyhow::Result<Vec<String>> {
    let Some((start, end)) = token.split_once('-') else {
        return Ok(vec![token.to_string()]);
    };
    let start = start.trim();
    let end = end.trim();
    if start.is_empty() || end.is_empty() {
        return Ok(vec![token.to_string()]);
    }
    let Some((prefix_start, start_num, start_width)) = split_waybill_suffix(start) else {
        return Ok(vec![token.to_string()]);
    };
    let Some((prefix_end, end_num, end_width)) = split_waybill_suffix(end) else {
        return Ok(vec![token.to_string()]);
    };
    if prefix_start != prefix_end {
        return Ok(vec![token.to_string()]);
    }
    if end_num < start_num {
        return Err(anyhow::anyhow!("invalid waybill range: {token}"));
    }
    let count = end_num - start_num + 1;
    if count > 10_000 {
        return Err(anyhow::anyhow!("waybill range too large: {token}"));
    }
    let width = start_width.max(end_width);
    let mut items = Vec::with_capacity(count as usize);
    for value in start_num..=end_num {
        items.push(format!("{}{:0width$}", prefix_start, value, width = width));
    }
    Ok(items)
}

pub(crate) fn parse_waybill_text(raw: &str) -> anyhow::Result<Vec<String>> {
    let mut items = Vec::new();
    for token in raw.split(|ch: char| {
        ch == '\n' || ch == '\r' || ch == ',' || ch == ';' || ch == '\t' || ch == ' '
    }) {
        let trimmed = token.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("waybill_no") {
            continue;
        }
        for expanded in expand_waybill_token(trimmed)? {
            let value = expanded.trim();
            if value.is_empty() {
                continue;
            }
            items.push(value.to_string());
        }
    }
    Ok(items)
}
