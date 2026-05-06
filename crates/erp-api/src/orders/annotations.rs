use super::*;

pub(super) async fn add_order_tags(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderTagsRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let tags: Vec<String> = req
        .tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();
    if tags.is_empty() {
        return bad_request("tags required").into_response();
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let created_tags = tags
        .iter()
        .map(|tag| OrderTagCreate {
            tag: tag.to_string(),
            created_by: Some(ctx.user_id),
        })
        .collect::<Vec<_>>();

    let result = match orders_repo
        .add_order_tags(tenant_id, order_id, created_tags)
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
            action: "OMS.ORDER_TAG_ADD".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "tags": tags })),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(result)).into_response()
}

fn build_tag_context(detail: &erp_repo::orders::OmsOrderDetail) -> OrderReviewContext {
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
    if let Some(phone) = detail.order.receiver_phone.as_ref() {
        context.insert_field("receiver_phone", serde_json::Value::String(phone.clone()));
    }
    if let Some(name) = detail.order.receiver_name.as_ref() {
        context.insert_field("receiver_name", serde_json::Value::String(name.clone()));
    }
    context.insert_field(
        "platform",
        serde_json::Value::String(detail.order.platform.to_string()),
    );
    context.insert_field(
        "shop_id",
        serde_json::Value::String(detail.order.shop_id.to_string()),
    );
    context.insert_field(
        "platform_order_id",
        serde_json::Value::String(detail.order.platform_order_id.clone()),
    );
    if let Some(address) = detail.order.receiver_address.as_ref() {
        context.insert_field(
            "receiver_address",
            serde_json::Value::String(address.clone()),
        );
        if address.trim().is_empty() {
            context.insert_field("address_invalid", serde_json::Value::Bool(true));
        }
    } else {
        context.insert_field("address_invalid", serde_json::Value::Bool(true));
    }
    context
}

pub(super) async fn apply_auto_tags(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderAutoTagApplyRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }
    if req.order_ids.is_empty() {
        return bad_request("order_ids required").into_response();
    }

    let tag_repo = OrderTagDefinitionsRepo::new(state.db.pool());
    let definitions = match tag_repo.list_auto_apply_tags(tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if definitions.is_empty() {
        return (
            StatusCode::OK,
            Json(OrderAutoTagApplyResponse {
                tagged_orders: Vec::new(),
                total: 0,
            }),
        )
            .into_response();
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let mut tagged_orders: Vec<String> = Vec::new();

    for raw_id in req.order_ids {
        let order_id = match OrderId::from_str(&raw_id) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

        let context = build_tag_context(&detail);
        let existing_tags: std::collections::HashSet<String> =
            detail.tags.iter().map(|tag| tag.tag.clone()).collect();
        let mut matched_tags: Vec<String> = Vec::new();

        for definition in &definitions {
            if existing_tags.contains(&definition.name) {
                continue;
            }
            let conditions: Vec<OrderReviewCondition> =
                match serde_json::from_value(definition.conditions_json.clone()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
            let matched = if conditions.is_empty() {
                true
            } else {
                evaluate_conditions(&context, &conditions)
            };
            if matched {
                matched_tags.push(definition.name.clone());
            }
        }

        if matched_tags.is_empty() {
            continue;
        }

        let created_tags = matched_tags
            .iter()
            .map(|tag| OrderTagCreate {
                tag: tag.to_string(),
                created_by: Some(ctx.user_id),
            })
            .collect::<Vec<_>>();
        if let Err(e) = orders_repo
            .add_order_tags(tenant_id, order_id, created_tags)
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }

        let audit_repo = AuditLogsRepo::new(state.db.pool());
        let _ = audit_repo
            .insert(AuditLogCreate {
                tenant_id,
                user_id: Some(ctx.user_id),
                action: "OMS.ORDER_TAG_AUTO_APPLY".to_string(),
                entity_type: "ORDER".to_string(),
                entity_id: Some(order_id.to_string()),
                before_json: None,
                after_json: Some(serde_json::json!({ "tags": matched_tags })),
                ip: None,
                ua: None,
            })
            .await;

        tagged_orders.push(order_id.to_string());
    }

    let response = OrderAutoTagApplyResponse {
        total: tagged_orders.len() as i64,
        tagged_orders,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn add_order_note(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderNoteRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    let note = req.note.trim();
    if note.is_empty() {
        return bad_request("note required").into_response();
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let result = match orders_repo
        .add_order_note(
            tenant_id,
            order_id,
            OrderNoteCreate {
                note: note.to_string(),
                created_by: Some(ctx.user_id),
            },
        )
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
            action: "OMS.ORDER_NOTE_ADD".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "note": note })),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(result)).into_response()
}

pub(crate) async fn list_order_tag_definitions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<TagDefinitionListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicOrderTagRead.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let repo = OrderTagDefinitionsRepo::new(state.db.pool());
    let result = match repo
        .list_tags(
            tenant_id,
            OrderTagDefinitionListFilters {
                keyword,
                status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = OrderTagDefinitionListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_order_tag_definition(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TagDefinitionCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicOrderTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let status = req
        .status
        .unwrap_or_else(|| "ACTIVE".to_string())
        .trim()
        .to_uppercase();
    if status.is_empty() {
        return bad_request("status required").into_response();
    }
    let conditions_json = req.conditions_json.unwrap_or_else(|| serde_json::json!([]));

    let repo = OrderTagDefinitionsRepo::new(state.db.pool());
    let tag = match repo
        .create_tag(OrderTagDefinitionCreate {
            tenant_id,
            name: name.to_string(),
            color: req.color.map(|value| value.trim().to_string()),
            priority: req.priority.unwrap_or(0),
            status,
            auto_apply: req.auto_apply.unwrap_or(false),
            conditions_json,
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
            action: "ORDER_TAG_DEF.CREATE".to_string(),
            entity_type: "order_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&tag).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

pub(crate) async fn update_order_tag_definition(
    State(state): State<AppState>,
    Path((tenant_id, tag_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TagDefinitionUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let tag_id = match uuid::Uuid::parse_str(&tag_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tag_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicOrderTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let color = req
        .color
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    if name.is_none()
        && color.is_none()
        && req.priority.is_none()
        && status.is_none()
        && req.auto_apply.is_none()
        && req.conditions_json.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let repo = OrderTagDefinitionsRepo::new(state.db.pool());
    let tag = match repo
        .update_tag(
            tenant_id,
            tag_id,
            OrderTagDefinitionUpdate {
                name,
                color,
                priority: req.priority,
                status,
                auto_apply: req.auto_apply,
                conditions_json: req.conditions_json,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order tag definition not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "ORDER_TAG_DEF.UPDATE".to_string(),
            entity_type: "order_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&tag).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

pub(crate) async fn delete_order_tag_definition(
    State(state): State<AppState>,
    Path((tenant_id, tag_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let tag_id = match uuid::Uuid::parse_str(&tag_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tag_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicOrderTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let repo = OrderTagDefinitionsRepo::new(state.db.pool());
    let usage = match repo.count_usage(tenant_id, tag_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if usage > 0 {
        return conflict("order tag in use").into_response();
    }

    let tag = match repo.delete_tag(tenant_id, tag_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order tag definition not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "ORDER_TAG_DEF.DELETE".to_string(),
            entity_type: "order_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: serde_json::to_value(&tag).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

pub(crate) async fn list_sku_tag_definitions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<TagDefinitionListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicSkuTagRead.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let repo = SkuTagDefinitionsRepo::new(state.db.pool());
    let result = match repo
        .list_tags(
            tenant_id,
            SkuTagDefinitionListFilters {
                keyword,
                status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = SkuTagDefinitionListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_sku_tag_definition(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TagDefinitionCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicSkuTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let status = req
        .status
        .unwrap_or_else(|| "ACTIVE".to_string())
        .trim()
        .to_uppercase();
    if status.is_empty() {
        return bad_request("status required").into_response();
    }

    let repo = SkuTagDefinitionsRepo::new(state.db.pool());
    let tag = match repo
        .create_tag(SkuTagDefinitionCreate {
            tenant_id,
            name: name.to_string(),
            color: req.color.map(|value| value.trim().to_string()),
            priority: req.priority.unwrap_or(0),
            status,
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
            action: "SKU_TAG_DEF.CREATE".to_string(),
            entity_type: "sku_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&tag).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

pub(crate) async fn update_sku_tag_definition(
    State(state): State<AppState>,
    Path((tenant_id, tag_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TagDefinitionUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let tag_id = match uuid::Uuid::parse_str(&tag_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tag_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicSkuTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let color = req
        .color
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    if name.is_none() && color.is_none() && req.priority.is_none() && status.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let repo = SkuTagDefinitionsRepo::new(state.db.pool());
    let tag = match repo
        .update_tag(
            tenant_id,
            tag_id,
            SkuTagDefinitionUpdate {
                name,
                color,
                priority: req.priority,
                status,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sku tag definition not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SKU_TAG_DEF.UPDATE".to_string(),
            entity_type: "sku_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&tag).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

pub(crate) async fn delete_sku_tag_definition(
    State(state): State<AppState>,
    Path((tenant_id, tag_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let tag_id = match uuid::Uuid::parse_str(&tag_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tag_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicSkuTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let repo = SkuTagDefinitionsRepo::new(state.db.pool());
    let tag = match repo.delete_tag(tenant_id, tag_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sku tag definition not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SKU_TAG_DEF.DELETE".to_string(),
            entity_type: "sku_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: serde_json::to_value(&tag).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}
