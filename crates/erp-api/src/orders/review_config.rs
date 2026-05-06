use super::*;

pub(super) async fn list_review_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OrderReviewRuleListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let filters = OrderReviewRuleListFilters {
        enabled: query.enabled,
        offset: query.offset.unwrap_or(0).max(0),
        limit: query.limit.unwrap_or(50).clamp(1, 200),
    };
    let repo = OrderReviewRulesRepo::new(state.db.pool());
    let result = match repo.list_rules(tenant_id, filters).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = OrderReviewRuleListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn get_review_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };

    let repo = OrderReviewRulesRepo::new(state.db.pool());
    match repo.get_rule(tenant_id, rule_id).await {
        Ok(Some(rule)) => (StatusCode::OK, Json(rule)).into_response(),
        Ok(None) => not_found("rule not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

pub(super) async fn create_review_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderReviewRuleRequest>,
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
    let conditions_json = req.conditions_json.unwrap_or_else(|| serde_json::json!([]));
    let repo = OrderReviewRulesRepo::new(state.db.pool());
    let rule = match repo
        .create_rule(OrderReviewRuleCreate {
            tenant_id,
            name: req.name,
            enabled: req.enabled.unwrap_or(true),
            priority: req.priority.unwrap_or(0),
            conditions_json,
            action: req.action,
            reason: req.reason.unwrap_or_default(),
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
            action: "OMS.REVIEW_RULE_CREATE".to_string(),
            entity_type: "ORDER_REVIEW_RULE".to_string(),
            entity_id: Some(rule.rule_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&rule).unwrap_or_default()),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(rule)).into_response()
}

pub(super) async fn update_review_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderReviewRuleUpdateRequest>,
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
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };

    let repo = OrderReviewRulesRepo::new(state.db.pool());
    let before = match repo.get_rule(tenant_id, rule_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(before) = before else {
        return not_found("rule not found").into_response();
    };

    let updated = match repo
        .update_rule(
            tenant_id,
            rule_id,
            OrderReviewRuleUpdate {
                name: req.name,
                enabled: req.enabled,
                priority: req.priority,
                conditions_json: req.conditions_json,
                action: req.action,
                reason: req.reason,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(updated) = updated else {
        return not_found("rule not found").into_response();
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.REVIEW_RULE_UPDATE".to_string(),
            entity_type: "ORDER_REVIEW_RULE".to_string(),
            entity_id: Some(rule_id.to_string()),
            before_json: Some(serde_json::to_value(before).unwrap_or_default()),
            after_json: Some(serde_json::to_value(&updated).unwrap_or_default()),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn delete_review_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };

    let repo = OrderReviewRulesRepo::new(state.db.pool());
    let deleted = match repo.delete_rule(tenant_id, rule_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(deleted) = deleted else {
        return not_found("rule not found").into_response();
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.REVIEW_RULE_DELETE".to_string(),
            entity_type: "ORDER_REVIEW_RULE".to_string(),
            entity_id: Some(rule_id.to_string()),
            before_json: Some(serde_json::to_value(&deleted).unwrap_or_default()),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(super) async fn list_exception_reasons(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ExceptionReasonListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let repo = ExceptionReasonsRepo::new(state.db.pool());
    let result = match repo
        .list_reasons(
            tenant_id,
            ExceptionReasonListFilters {
                category: query.category,
                offset: query.offset.unwrap_or(0).max(0),
                limit: query.limit.unwrap_or(100).clamp(1, 500),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ExceptionReasonListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn list_customer_blacklist(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CustomerBlacklistListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let repo = CustomerBlacklistRepo::new(state.db.pool());
    let result = match repo
        .list_entries(
            tenant_id,
            CustomerBlacklistListFilters {
                key_type: query.key_type,
                keyword: query.keyword,
                offset: query.offset.unwrap_or(0).max(0),
                limit: query.limit.unwrap_or(100).clamp(1, 500),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = CustomerBlacklistListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn create_customer_blacklist(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CustomerBlacklistRequest>,
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

    let repo = CustomerBlacklistRepo::new(state.db.pool());
    let entry = match repo
        .create_entry(CustomerBlacklistCreate {
            tenant_id,
            key_type: req.key_type,
            key_value: req.key_value,
            reason: req.reason.unwrap_or_default(),
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
            action: "OMS.BLACKLIST_CREATE".to_string(),
            entity_type: "CUSTOMER_BLACKLIST".to_string(),
            entity_id: Some(entry.blacklist_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&entry).unwrap_or_default()),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(entry)).into_response()
}

pub(super) async fn delete_customer_blacklist(
    State(state): State<AppState>,
    Path((tenant_id, blacklist_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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
    let blacklist_id = match uuid::Uuid::parse_str(&blacklist_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad blacklist_id").into_response(),
    };

    let repo = CustomerBlacklistRepo::new(state.db.pool());
    let entry = match repo.delete_entry(tenant_id, blacklist_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(entry) = entry else {
        return not_found("blacklist entry not found").into_response();
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.BLACKLIST_DELETE".to_string(),
            entity_type: "CUSTOMER_BLACKLIST".to_string(),
            entity_id: Some(entry.blacklist_id.to_string()),
            before_json: Some(serde_json::to_value(&entry).unwrap_or_default()),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(entry)).into_response()
}
