use super::*;

fn shared_partner_request_id(headers: &HeaderMap, body: Option<&str>) -> Result<Option<String>, Response> {
    let header_key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let body_key = body.map(str::trim).filter(|value| !value.is_empty());
    match (header_key, body_key) {
        (Some(header), Some(body)) if header != body => {
            Err(conflict("Idempotency-Key disagrees with request_id").into_response())
        }
        (Some(header), _) => Ok(Some(header.to_string())),
        (None, Some(body)) => Ok(Some(body.to_string())),
        (None, None) => Ok(None),
    }
}

async fn link_shared_partner(
    state: &AppState,
    headers: &HeaderMap,
    tenant_id: TenantId,
    body_request_id: Option<&str>,
    name: &str,
    customer: bool,
    supplier: bool,
    customer_no: &str,
    supplier_no: &str,
    term_days: i32,
    tax_no: &str,
    address: &str,
    contact: &str,
) -> Result<Option<serde_json::Value>, Response> {
    let Some(request_id) = shared_partner_request_id(headers, body_request_id)? else {
        return Ok(None);
    };
    let repo = erp_repo::shared_contract::SharedContractRepo::new(state.db.pool());
    let saved = repo
        .save_shared_partner(erp_repo::shared_contract::SharedPartnerInput { tenant_id, request_id: &request_id, payload_hash: &request_id, name, customer, supplier, customer_no, supplier_no, term_days, tax_no })
        .await;
    let (write, partner_id) = match saved {
        Ok(Ok((write, partner_id))) => (write, partner_id),
        Ok(Err(_)) => return Err(conflict("payload conflict").into_response()),
        Err(error) => return Err((StatusCode::CONFLICT, format!("{error}")).into_response()),
    };
    let (receipt, replay) = match write {
        erp_repo::shared_contract::SharedWrite::Stored(receipt) => (receipt, false),
        erp_repo::shared_contract::SharedWrite::Replay(receipt) => (receipt, true),
    };
    if partner_id.to_string() == customer_no || partner_id.to_string() == supplier_no {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "partner id reused a role number").into_response());
    }
    Ok(Some(serde_json::json!({
        "request_id": receipt.request_id,
        "status": receipt.status.as_str(),
        "replay": replay,
        "partner_id": partner_id,
        "customer": customer,
        "supplier": supplier,
        "customer_no": customer_no,
        "supplier_no": supplier_no,
        "term_days": term_days,
        "tax_no": tax_no,
        "address": address,
        "contact": contact,
    })))
}

async fn refuse_referenced_partner(
    state: &AppState,
    tenant_id: TenantId,
    partner_id: uuid::Uuid,
) -> Result<(), Response> {
    let referenced = match sqlx::query_scalar::<_, i64>(
        "select count(*) from shared_partner_refs where tenant_id = $1 and partner_id = $2",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(partner_id)
    .fetch_one(state.db.pool())
    .await
    {
        Ok(count) => count,
        Err(error) if error.to_string().contains("shared_partner_refs") => 0,
        Err(error) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()),
    };
    if referenced > 0 {
        return Err(conflict("referenced partner cannot be deleted").into_response());
    }
    Ok(())
}

pub(crate) async fn list_suppliers(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SupplierListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "settings.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let result = match state
        .suppliers_repo
        .list_suppliers(
            tenant_id,
            SupplierListFilters {
                keyword,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = SupplierListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_supplier(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SupplierCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "settings.read") {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }

    let supplier = match state
        .suppliers_repo
        .create_supplier(SupplierCreate {
            tenant_id,
            name: name.to_string(),
            contact: req.contact.clone(),
            settlement_terms: req.settlement_terms.clone(),
            status: req.status.clone().unwrap_or_else(|| "ACTIVE".to_string()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SUPPLIERS.CREATE".to_string(),
            entity_type: "supplier".to_string(),
            entity_id: Some(supplier.supplier_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&supplier).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write supplier audit log");
    }

    let supplier_no = req.supplier_no.unwrap_or_else(|| supplier.supplier_id.to_string());
    let customer_no = req.customer_no.unwrap_or_default();
    match link_shared_partner(
        &state,
        &headers,
        tenant_id,
        req.request_id.as_deref(),
        &supplier.name,
        !customer_no.trim().is_empty(),
        true,
        &customer_no,
        &supplier_no,
        req.term_days.unwrap_or(0),
        req.tax_no.as_deref().unwrap_or(""),
        "",
        supplier.contact.as_deref().unwrap_or(""),
    )
    .await
    {
        Ok(Some(link)) => (StatusCode::OK, Json(link)).into_response(),
        Ok(None) => (StatusCode::OK, Json(supplier)).into_response(),
        Err(resp) => resp,
    }
}

pub(crate) async fn update_supplier(
    State(state): State<AppState>,
    Path((tenant_id, supplier_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SupplierUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let supplier_id = match uuid::Uuid::parse_str(&supplier_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad supplier_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "suppliers.write") {
        return resp.into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let contact = req
        .contact
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let settlement_terms = req
        .settlement_terms
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if name.is_none() && contact.is_none() && settlement_terms.is_none() && status.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let supplier = match state
        .suppliers_repo
        .update_supplier(
            tenant_id,
            supplier_id,
            SupplierUpdate {
                name,
                contact,
                settlement_terms,
                status,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("supplier not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SUPPLIERS.UPDATE".to_string(),
            entity_type: "supplier".to_string(),
            entity_id: Some(supplier.supplier_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&supplier).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write supplier audit log");
    }

    (StatusCode::OK, Json(supplier)).into_response()
}

pub(crate) async fn delete_supplier(
    State(state): State<AppState>,
    Path((tenant_id, supplier_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let supplier_id = match uuid::Uuid::parse_str(&supplier_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad supplier_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "settings.read") {
        return resp.into_response();
    }
    if let Err(resp) = refuse_referenced_partner(&state, tenant_id, supplier_id).await {
        return resp;
    }

    let supplier = match state
        .suppliers_repo
        .delete_supplier(tenant_id, supplier_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("supplier not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SUPPLIERS.DELETE".to_string(),
            entity_type: "supplier".to_string(),
            entity_id: Some(supplier.supplier_id.to_string()),
            before_json: serde_json::to_value(&supplier).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write supplier audit log");
    }

    (StatusCode::OK, Json(supplier)).into_response()
}

pub(crate) async fn list_customer_categories(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CustomerCategoryListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.read") {
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
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let repo = CustomerCategoriesRepo::new(state.db.pool());
    let result = match repo
        .list_categories(
            tenant_id,
            CustomerCategoryListFilters {
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

    let response = CustomerCategoryListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_customer_category(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CustomerCategoryCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }
    let description = req
        .description
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .unwrap_or_else(|| "ACTIVE".to_string())
        .trim()
        .to_uppercase();
    if status.is_empty() {
        return bad_request("status is required").into_response();
    }

    let repo = CustomerCategoriesRepo::new(state.db.pool());
    let category = match repo
        .create_category(CustomerCategoryCreate {
            tenant_id,
            name: name.to_string(),
            description,
            status,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CUSTOMER_CATEGORY.CREATE".to_string(),
            entity_type: "customer_category".to_string(),
            entity_id: Some(category.category_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&category).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write customer category audit log");
    }

    (StatusCode::OK, Json(category)).into_response()
}

pub(crate) async fn update_customer_category(
    State(state): State<AppState>,
    Path((tenant_id, category_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CustomerCategoryUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let category_id = match uuid::Uuid::parse_str(&category_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad category_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let description = req
        .description
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    if name.is_none() && description.is_none() && status.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let repo = CustomerCategoriesRepo::new(state.db.pool());
    let category = match repo
        .update_category(
            tenant_id,
            category_id,
            CustomerCategoryUpdate {
                name,
                description,
                status,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("customer category not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CUSTOMER_CATEGORY.UPDATE".to_string(),
            entity_type: "customer_category".to_string(),
            entity_id: Some(category.category_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&category).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write customer category audit log");
    }

    (StatusCode::OK, Json(category)).into_response()
}

pub(crate) async fn delete_customer_category(
    State(state): State<AppState>,
    Path((tenant_id, category_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let category_id = match uuid::Uuid::parse_str(&category_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad category_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let repo = CustomerCategoriesRepo::new(state.db.pool());
    let category = match repo.delete_category(tenant_id, category_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("customer category not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CUSTOMER_CATEGORY.DELETE".to_string(),
            entity_type: "customer_category".to_string(),
            entity_id: Some(category.category_id.to_string()),
            before_json: serde_json::to_value(&category).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write customer category audit log");
    }

    (StatusCode::OK, Json(category)).into_response()
}

pub(crate) async fn list_member_levels(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<MemberLevelListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.read") {
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
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let repo = MemberLevelsRepo::new(state.db.pool());
    let result = match repo
        .list_levels(
            tenant_id,
            MemberLevelListFilters {
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

    let response = MemberLevelListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_member_level(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<MemberLevelCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }
    let rules = req
        .rules
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .unwrap_or_else(|| "ACTIVE".to_string())
        .trim()
        .to_uppercase();
    if status.is_empty() {
        return bad_request("status is required").into_response();
    }

    let repo = MemberLevelsRepo::new(state.db.pool());
    let level = match repo
        .create_level(MemberLevelCreate {
            tenant_id,
            name: name.to_string(),
            level_rank: req.level_rank.unwrap_or(0),
            discount_rate: req.discount_rate,
            rules,
            status,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "MEMBER_LEVEL.CREATE".to_string(),
            entity_type: "member_level".to_string(),
            entity_id: Some(level.level_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&level).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write member level audit log");
    }

    (StatusCode::OK, Json(level)).into_response()
}

pub(crate) async fn update_member_level(
    State(state): State<AppState>,
    Path((tenant_id, level_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<MemberLevelUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let level_id = match uuid::Uuid::parse_str(&level_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad level_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let rules = req
        .rules
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let level_rank = req.level_rank;
    let discount_rate = req.discount_rate;
    if name.is_none()
        && rules.is_none()
        && status.is_none()
        && level_rank.is_none()
        && discount_rate.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let repo = MemberLevelsRepo::new(state.db.pool());
    let level = match repo
        .update_level(
            tenant_id,
            level_id,
            MemberLevelUpdate {
                name,
                level_rank,
                discount_rate,
                rules,
                status,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("member level not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "MEMBER_LEVEL.UPDATE".to_string(),
            entity_type: "member_level".to_string(),
            entity_id: Some(level.level_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&level).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write member level audit log");
    }

    (StatusCode::OK, Json(level)).into_response()
}

pub(crate) async fn delete_member_level(
    State(state): State<AppState>,
    Path((tenant_id, level_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let level_id = match uuid::Uuid::parse_str(&level_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad level_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let repo = MemberLevelsRepo::new(state.db.pool());
    let level = match repo.delete_level(tenant_id, level_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("member level not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "MEMBER_LEVEL.DELETE".to_string(),
            entity_type: "member_level".to_string(),
            entity_id: Some(level.level_id.to_string()),
            before_json: serde_json::to_value(&level).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write member level audit log");
    }

    (StatusCode::OK, Json(level)).into_response()
}

pub(crate) async fn list_customer_tags(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CustomerTagListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let repo = CustomerTagsRepo::new(state.db.pool());
    let result = match repo
        .list_tags(
            tenant_id,
            CustomerTagListFilters {
                keyword,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = CustomerTagListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_customer_tag(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CustomerTagCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }
    let color = req
        .color
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let repo = CustomerTagsRepo::new(state.db.pool());
    let tag = match repo
        .create_tag(CustomerTagCreate {
            tenant_id,
            name: name.to_string(),
            color,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CUSTOMER_TAG.CREATE".to_string(),
            entity_type: "customer_tag".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&tag).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write customer tag audit log");
    }

    (StatusCode::OK, Json(tag)).into_response()
}

pub(crate) async fn update_customer_tag(
    State(state): State<AppState>,
    Path((tenant_id, tag_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CustomerTagUpdateRequest>,
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
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let color = req
        .color
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if name.is_none() && color.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let repo = CustomerTagsRepo::new(state.db.pool());
    let tag = match repo
        .update_tag(tenant_id, tag_id, CustomerTagUpdate { name, color })
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("customer tag not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CUSTOMER_TAG.UPDATE".to_string(),
            entity_type: "customer_tag".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&tag).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write customer tag audit log");
    }

    (StatusCode::OK, Json(tag)).into_response()
}

pub(crate) async fn delete_customer_tag(
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
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let repo = CustomerTagsRepo::new(state.db.pool());
    let tag = match repo.delete_tag(tenant_id, tag_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("customer tag not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CUSTOMER_TAG.DELETE".to_string(),
            entity_type: "customer_tag".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: serde_json::to_value(&tag).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write customer tag audit log");
    }

    (StatusCode::OK, Json(tag)).into_response()
}

pub(crate) async fn list_customers(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CustomerListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.read") {
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
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let category_id = match query.category_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad category_id").into_response(),
        },
        None => None,
    };
    let member_level_id = match query.member_level_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad member_level_id").into_response(),
        },
        None => None,
    };
    let tag_id = match query.tag_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad tag_id").into_response(),
        },
        None => None,
    };

    let repo = CustomersRepo::new(state.db.pool());
    let result = match repo
        .list_customers(
            tenant_id,
            CustomerListFilters {
                keyword,
                status,
                category_id,
                member_level_id,
                tag_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = CustomerListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_customer_detail(
    State(state): State<AppState>,
    Path((tenant_id, customer_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let customer_id = match uuid::Uuid::parse_str(&customer_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad customer_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.read") {
        return resp.into_response();
    }

    let repo = CustomersRepo::new(state.db.pool());
    let detail = match repo.get_customer_detail(tenant_id, customer_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("customer not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = CustomerDetailResponse {
        customer: detail.customer,
        tags: detail.tags,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_customer(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CustomerCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }
    let phone = req
        .phone
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let address = req
        .address
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let note = req
        .note
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .unwrap_or_else(|| "ACTIVE".to_string())
        .trim()
        .to_uppercase();
    if status.is_empty() {
        return bad_request("status is required").into_response();
    }
    let category_id = match req.category_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad category_id").into_response(),
        },
        None => None,
    };
    let member_level_id = match req.member_level_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad member_level_id").into_response(),
        },
        None => None,
    };

    let repo = CustomersRepo::new(state.db.pool());
    let customer = match repo
        .create_customer(CustomerCreate {
            tenant_id,
            name: name.to_string(),
            phone,
            address,
            note,
            status,
            category_id,
            member_level_id,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CUSTOMER.CREATE".to_string(),
            entity_type: "customer".to_string(),
            entity_id: Some(customer.customer_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&customer).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write customer audit log");
    }

    let customer_no = req.customer_no.unwrap_or_else(|| customer.customer_id.to_string());
    let supplier_no = req.supplier_no.unwrap_or_default();
    match link_shared_partner(
        &state,
        &headers,
        tenant_id,
        req.request_id.as_deref(),
        &customer.name,
        true,
        !supplier_no.trim().is_empty(),
        &customer_no,
        &supplier_no,
        req.term_days.unwrap_or(0),
        req.tax_no.as_deref().unwrap_or(""),
        &customer.address.clone().unwrap_or_default(),
        &customer.phone.clone().unwrap_or_default(),
    )
    .await
    {
        Ok(Some(link)) => (StatusCode::OK, Json(link)).into_response(),
        Ok(None) => (StatusCode::OK, Json(customer)).into_response(),
        Err(resp) => resp,
    }
}

pub(crate) async fn update_customer(
    State(state): State<AppState>,
    Path((tenant_id, customer_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CustomerUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let customer_id = match uuid::Uuid::parse_str(&customer_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad customer_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let phone = req
        .phone
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let address = req
        .address
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let note = req
        .note
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let category_id = match req.category_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad category_id").into_response(),
        },
        None => None,
    };
    let member_level_id = match req.member_level_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad member_level_id").into_response(),
        },
        None => None,
    };
    if name.is_none()
        && phone.is_none()
        && address.is_none()
        && note.is_none()
        && status.is_none()
        && category_id.is_none()
        && member_level_id.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let repo = CustomersRepo::new(state.db.pool());
    let customer = match repo
        .update_customer(
            tenant_id,
            customer_id,
            CustomerUpdate {
                name,
                phone,
                address,
                note,
                status,
                category_id,
                member_level_id,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("customer not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CUSTOMER.UPDATE".to_string(),
            entity_type: "customer".to_string(),
            entity_id: Some(customer.customer_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&customer).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write customer audit log");
    }

    (StatusCode::OK, Json(customer)).into_response()
}

pub(crate) async fn delete_customer(
    State(state): State<AppState>,
    Path((tenant_id, customer_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let customer_id = match uuid::Uuid::parse_str(&customer_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad customer_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }
    if let Err(resp) = refuse_referenced_partner(&state, tenant_id, customer_id).await {
        return resp;
    }

    let repo = CustomersRepo::new(state.db.pool());
    let customer = match repo.delete_customer(tenant_id, customer_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("customer not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CUSTOMER.DELETE".to_string(),
            entity_type: "customer".to_string(),
            entity_id: Some(customer.customer_id.to_string()),
            before_json: serde_json::to_value(&customer).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write customer audit log");
    }

    (StatusCode::OK, Json(customer)).into_response()
}

pub(crate) async fn replace_customer_tags(
    State(state): State<AppState>,
    Path((tenant_id, customer_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CustomerTagReplaceRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let customer_id = match uuid::Uuid::parse_str(&customer_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad customer_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let tag_ids = req
        .tag_ids
        .iter()
        .map(|raw| uuid::Uuid::parse_str(raw).map_err(|_| bad_request("bad tag_id")))
        .collect::<Result<Vec<_>, _>>();
    let tag_ids = match tag_ids {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let repo = CustomersRepo::new(state.db.pool());
    let exists = match repo.get_customer(tenant_id, customer_id).await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if !exists {
        return not_found("customer not found").into_response();
    }

    let tags = match repo
        .replace_customer_tags(tenant_id, customer_id, tag_ids)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(tags)).into_response()
}

pub(crate) async fn list_crm_customer_segments(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CrmCustomerSegmentListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let segment_type = query
        .segment_type
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    let status = query
        .status
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());

    let tenant_uuid = uuid::Uuid::from(tenant_id);
    let total = match sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)::bigint
        from crm_customer_segments
        where tenant_id = $1
          and ($2::text is null
               or name ilike '%' || $2 || '%'
               or coalesce(rule_summary, '') ilike '%' || $2 || '%')
          and ($3::text is null or segment_type = $3)
          and ($4::text is null or status = $4)
        "#,
    )
    .bind(tenant_uuid)
    .bind(keyword.clone())
    .bind(segment_type.clone())
    .bind(status.clone())
    .fetch_one(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items: Vec<CrmCustomerSegment> = match sqlx::query_as(
        r#"
        select
          segment_id,
          name,
          segment_type,
          rule_summary,
          customer_count,
          status,
          created_at,
          updated_at
        from crm_customer_segments
        where tenant_id = $1
          and ($2::text is null
               or name ilike '%' || $2 || '%'
               or coalesce(rule_summary, '') ilike '%' || $2 || '%')
          and ($3::text is null or segment_type = $3)
          and ($4::text is null or status = $4)
        order by updated_at desc, segment_id desc
        offset $5 limit $6
        "#,
    )
    .bind(tenant_uuid)
    .bind(keyword)
    .bind(segment_type)
    .bind(status)
    .bind(offset)
    .bind(limit)
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(CrmCustomerSegmentListResponse { items, total }),
    )
        .into_response()
}

pub(crate) async fn create_crm_customer_segment(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CrmCustomerSegmentCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }
    let segment_type = req.segment_type.trim().to_uppercase();
    if !matches!(segment_type.as_str(), "DYNAMIC" | "STATIC") {
        return bad_request("bad segment_type").into_response();
    }
    let status = req
        .status
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "ACTIVE".to_string());
    let rule_summary = req
        .rule_summary
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let segment: CrmCustomerSegment = match sqlx::query_as(
        r#"
        insert into crm_customer_segments(
          tenant_id,
          name,
          segment_type,
          rule_summary,
          status
        ) values ($1, $2, $3, $4, $5)
        returning
          segment_id,
          name,
          segment_type,
          rule_summary,
          customer_count,
          status,
          created_at,
          updated_at
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(name.to_string())
    .bind(segment_type)
    .bind(rule_summary)
    .bind(status)
    .fetch_one(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(segment)).into_response()
}

pub(crate) async fn update_crm_customer_segment(
    State(state): State<AppState>,
    Path((tenant_id, segment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CrmCustomerSegmentUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let segment_id = match uuid::Uuid::parse_str(&segment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad segment_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let name = req
        .name
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let segment_type = req
        .segment_type
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    if let Some(value) = segment_type.as_deref() {
        if !matches!(value, "DYNAMIC" | "STATIC") {
            return bad_request("bad segment_type").into_response();
        }
    }
    let rule_summary = req
        .rule_summary
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let status = req
        .status
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    if name.is_none() && segment_type.is_none() && rule_summary.is_none() && status.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let segment = match sqlx::query_as::<_, CrmCustomerSegment>(
        r#"
        update crm_customer_segments
        set
          name = coalesce($3, name),
          segment_type = coalesce($4, segment_type),
          rule_summary = coalesce($5, rule_summary),
          status = coalesce($6, status),
          updated_at = now()
        where tenant_id = $1 and segment_id = $2
        returning
          segment_id,
          name,
          segment_type,
          rule_summary,
          customer_count,
          status,
          created_at,
          updated_at
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(segment_id)
    .bind(name)
    .bind(segment_type)
    .bind(rule_summary)
    .bind(status)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("customer segment not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(segment)).into_response()
}

pub(crate) async fn delete_crm_customer_segment(
    State(state): State<AppState>,
    Path((tenant_id, segment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let segment_id = match uuid::Uuid::parse_str(&segment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad segment_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let segment = match sqlx::query_as::<_, CrmCustomerSegment>(
        r#"
        delete from crm_customer_segments
        where tenant_id = $1 and segment_id = $2
        returning
          segment_id,
          name,
          segment_type,
          rule_summary,
          customer_count,
          status,
          created_at,
          updated_at
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(segment_id)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("customer segment not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(segment)).into_response()
}

pub(crate) async fn list_crm_lifecycle_tasks(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CrmLifecycleTaskListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let stage = query
        .stage
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    let status = query
        .status
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    let tenant_uuid = uuid::Uuid::from(tenant_id);

    let total = match sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)::bigint
        from crm_lifecycle_tasks
        where tenant_id = $1
          and ($2::text is null
               or strategy ilike '%' || $2 || '%'
               or coalesce(owner, '') ilike '%' || $2 || '%'
               or coalesce(trigger_rule, '') ilike '%' || $2 || '%')
          and ($3::text is null or stage = $3)
          and ($4::text is null or status = $4)
        "#,
    )
    .bind(tenant_uuid)
    .bind(keyword.clone())
    .bind(stage.clone())
    .bind(status.clone())
    .fetch_one(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items: Vec<CrmLifecycleTask> = match sqlx::query_as(
        r#"
        select
          task_id,
          stage,
          strategy,
          trigger_rule,
          owner,
          status,
          last_triggered_at,
          created_at,
          updated_at
        from crm_lifecycle_tasks
        where tenant_id = $1
          and ($2::text is null
               or strategy ilike '%' || $2 || '%'
               or coalesce(owner, '') ilike '%' || $2 || '%'
               or coalesce(trigger_rule, '') ilike '%' || $2 || '%')
          and ($3::text is null or stage = $3)
          and ($4::text is null or status = $4)
        order by updated_at desc, task_id desc
        offset $5 limit $6
        "#,
    )
    .bind(tenant_uuid)
    .bind(keyword)
    .bind(stage)
    .bind(status)
    .bind(offset)
    .bind(limit)
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(CrmLifecycleTaskListResponse { items, total }),
    )
        .into_response()
}

pub(crate) async fn create_crm_lifecycle_task(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CrmLifecycleTaskCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let stage = req.stage.trim().to_uppercase();
    if !matches!(stage.as_str(), "NEW" | "ACTIVE" | "DORMANT" | "CHURNED") {
        return bad_request("bad stage").into_response();
    }
    let strategy = req.strategy.trim();
    if strategy.is_empty() {
        return bad_request("strategy is required").into_response();
    }
    let status = req
        .status
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "ACTIVE".to_string());
    let trigger_rule = req
        .trigger_rule
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let owner = req
        .owner
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let task: CrmLifecycleTask = match sqlx::query_as(
        r#"
        insert into crm_lifecycle_tasks(
          tenant_id,
          stage,
          strategy,
          trigger_rule,
          owner,
          status
        ) values ($1, $2, $3, $4, $5, $6)
        returning
          task_id,
          stage,
          strategy,
          trigger_rule,
          owner,
          status,
          last_triggered_at,
          created_at,
          updated_at
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(stage)
    .bind(strategy.to_string())
    .bind(trigger_rule)
    .bind(owner)
    .bind(status)
    .fetch_one(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(task)).into_response()
}

pub(crate) async fn update_crm_lifecycle_task(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CrmLifecycleTaskUpdateRequest>,
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
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let stage = req
        .stage
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    if let Some(value) = stage.as_deref() {
        if !matches!(value, "NEW" | "ACTIVE" | "DORMANT" | "CHURNED") {
            return bad_request("bad stage").into_response();
        }
    }
    let strategy = req
        .strategy
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let trigger_rule = req
        .trigger_rule
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let owner = req
        .owner
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let status = req
        .status
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    if stage.is_none()
        && strategy.is_none()
        && trigger_rule.is_none()
        && owner.is_none()
        && status.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let task = match sqlx::query_as::<_, CrmLifecycleTask>(
        r#"
        update crm_lifecycle_tasks
        set
          stage = coalesce($3, stage),
          strategy = coalesce($4, strategy),
          trigger_rule = coalesce($5, trigger_rule),
          owner = coalesce($6, owner),
          status = coalesce($7, status),
          updated_at = now()
        where tenant_id = $1 and task_id = $2
        returning
          task_id,
          stage,
          strategy,
          trigger_rule,
          owner,
          status,
          last_triggered_at,
          created_at,
          updated_at
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(task_id)
    .bind(stage)
    .bind(strategy)
    .bind(trigger_rule)
    .bind(owner)
    .bind(status)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("lifecycle task not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(task)).into_response()
}

pub(crate) async fn delete_crm_lifecycle_task(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let task = match sqlx::query_as::<_, CrmLifecycleTask>(
        r#"
        delete from crm_lifecycle_tasks
        where tenant_id = $1 and task_id = $2
        returning
          task_id,
          stage,
          strategy,
          trigger_rule,
          owner,
          status,
          last_triggered_at,
          created_at,
          updated_at
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(task_id)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("lifecycle task not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(task)).into_response()
}

pub(crate) async fn list_crm_service_tickets(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CrmServiceTicketListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.read") {
        return resp.into_response();
    }

    let updated_from = match query.updated_from {
        Some(raw) => match DateTime::parse_from_rfc3339(raw.trim()) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad updated_from").into_response(),
        },
        None => None,
    };
    let updated_to = match query.updated_to {
        Some(raw) => match DateTime::parse_from_rfc3339(raw.trim()) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad updated_to").into_response(),
        },
        None => None,
    };
    if let (Some(from), Some(to)) = (updated_from, updated_to) {
        if from > to {
            return bad_request("updated_from after updated_to").into_response();
        }
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let status = query
        .status
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    let priority = query
        .priority
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    let channel = query
        .channel
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let category = query
        .category
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let handler = query
        .handler
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let tenant_uuid = uuid::Uuid::from(tenant_id);
    let total = match sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)::bigint
        from crm_service_tickets
        where tenant_id = $1
          and ($2::text is null
               or customer_name ilike '%' || $2 || '%'
               or ticket_id::text ilike '%' || $2 || '%'
               or coalesce(related_order_id, '') ilike '%' || $2 || '%')
          and ($3::text is null or status = $3)
          and ($4::text is null or priority = $4)
          and ($5::text is null or channel = $5)
          and ($6::text is null or category = $6)
          and ($7::text is null or coalesce(handler, '') = $7)
          and ($8::timestamptz is null or updated_at >= $8)
          and ($9::timestamptz is null or updated_at <= $9)
        "#,
    )
    .bind(tenant_uuid)
    .bind(keyword.clone())
    .bind(status.clone())
    .bind(priority.clone())
    .bind(channel.clone())
    .bind(category.clone())
    .bind(handler.clone())
    .bind(updated_from)
    .bind(updated_to)
    .fetch_one(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items: Vec<CrmServiceTicket> = match sqlx::query_as(
        r#"
        select
          ticket_id,
          customer_id,
          customer_name,
          contact_phone,
          channel,
          category,
          priority,
          status,
          related_order_id,
          satisfaction_score,
          handler,
          description,
          created_at,
          updated_at
        from crm_service_tickets
        where tenant_id = $1
          and ($2::text is null
               or customer_name ilike '%' || $2 || '%'
               or ticket_id::text ilike '%' || $2 || '%'
               or coalesce(related_order_id, '') ilike '%' || $2 || '%')
          and ($3::text is null or status = $3)
          and ($4::text is null or priority = $4)
          and ($5::text is null or channel = $5)
          and ($6::text is null or category = $6)
          and ($7::text is null or coalesce(handler, '') = $7)
          and ($8::timestamptz is null or updated_at >= $8)
          and ($9::timestamptz is null or updated_at <= $9)
        order by updated_at desc, ticket_id desc
        offset $10 limit $11
        "#,
    )
    .bind(tenant_uuid)
    .bind(keyword)
    .bind(status)
    .bind(priority)
    .bind(channel)
    .bind(category)
    .bind(handler)
    .bind(updated_from)
    .bind(updated_to)
    .bind(offset)
    .bind(limit)
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(CrmServiceTicketListResponse { items, total }),
    )
        .into_response()
}

pub(crate) async fn create_crm_service_ticket(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CrmServiceTicketCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let customer_name = req.customer_name.trim();
    if customer_name.is_empty() {
        return bad_request("customer_name is required").into_response();
    }
    if let Some(score) = req.satisfaction_score {
        if !(1..=5).contains(&score) {
            return bad_request("bad satisfaction_score").into_response();
        }
    }

    let ticket: CrmServiceTicket = match sqlx::query_as(
        r#"
        insert into crm_service_tickets(
          tenant_id,
          customer_id,
          customer_name,
          contact_phone,
          channel,
          category,
          priority,
          status,
          related_order_id,
          satisfaction_score,
          handler,
          description
        ) values (
          $1, $2, $3, $4, $5, $6, $7, coalesce($8, 'OPEN'), $9, $10, $11, $12
        )
        returning
          ticket_id,
          customer_id,
          customer_name,
          contact_phone,
          channel,
          category,
          priority,
          status,
          related_order_id,
          satisfaction_score,
          handler,
          description,
          created_at,
          updated_at
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(req.customer_id.and_then(non_empty))
    .bind(customer_name.to_string())
    .bind(req.contact_phone.and_then(non_empty))
    .bind(req.channel.and_then(non_empty))
    .bind(req.category.and_then(non_empty))
    .bind(req.priority.map(|v| v.trim().to_uppercase()))
    .bind(req.status.map(|v| v.trim().to_uppercase()))
    .bind(req.related_order_id.and_then(non_empty))
    .bind(req.satisfaction_score)
    .bind(req.handler.and_then(non_empty))
    .bind(req.description.and_then(non_empty))
    .fetch_one(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(ticket)).into_response()
}

pub(crate) async fn update_crm_service_ticket(
    State(state): State<AppState>,
    Path((tenant_id, ticket_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CrmServiceTicketUpdateRequest>,
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
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }
    if let Some(score) = req.satisfaction_score {
        if !(1..=5).contains(&score) {
            return bad_request("bad satisfaction_score").into_response();
        }
    }

    let customer_id = req.customer_id.and_then(non_empty);
    let customer_name = req.customer_name.and_then(non_empty);
    let contact_phone = req.contact_phone.and_then(non_empty);
    let channel = req.channel.and_then(non_empty);
    let category = req.category.and_then(non_empty);
    let priority = req
        .priority
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    let status = req
        .status
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    let related_order_id = req.related_order_id.and_then(non_empty);
    let handler = req.handler.and_then(non_empty);
    let description = req.description.and_then(non_empty);

    if customer_id.is_none()
        && customer_name.is_none()
        && contact_phone.is_none()
        && channel.is_none()
        && category.is_none()
        && priority.is_none()
        && status.is_none()
        && related_order_id.is_none()
        && req.satisfaction_score.is_none()
        && handler.is_none()
        && description.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let ticket = match sqlx::query_as::<_, CrmServiceTicket>(
        r#"
        update crm_service_tickets
        set
          customer_id = coalesce($3, customer_id),
          customer_name = coalesce($4, customer_name),
          contact_phone = coalesce($5, contact_phone),
          channel = coalesce($6, channel),
          category = coalesce($7, category),
          priority = coalesce($8, priority),
          status = coalesce($9, status),
          related_order_id = coalesce($10, related_order_id),
          satisfaction_score = coalesce($11, satisfaction_score),
          handler = coalesce($12, handler),
          description = coalesce($13, description),
          updated_at = now()
        where tenant_id = $1 and ticket_id = $2
        returning
          ticket_id,
          customer_id,
          customer_name,
          contact_phone,
          channel,
          category,
          priority,
          status,
          related_order_id,
          satisfaction_score,
          handler,
          description,
          created_at,
          updated_at
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(ticket_id)
    .bind(customer_id)
    .bind(customer_name)
    .bind(contact_phone)
    .bind(channel)
    .bind(category)
    .bind(priority)
    .bind(status)
    .bind(related_order_id)
    .bind(req.satisfaction_score)
    .bind(handler)
    .bind(description)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("service ticket not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(ticket)).into_response()
}

pub(crate) async fn delete_crm_service_ticket(
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
    if let Err(resp) = ensure_permission(&ctx, "customers.write") {
        return resp.into_response();
    }

    let ticket = match sqlx::query_as::<_, CrmServiceTicket>(
        r#"
        delete from crm_service_tickets
        where tenant_id = $1 and ticket_id = $2
        returning
          ticket_id,
          customer_id,
          customer_name,
          contact_phone,
          channel,
          category,
          priority,
          status,
          related_order_id,
          satisfaction_score,
          handler,
          description,
          created_at,
          updated_at
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(ticket_id)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("service ticket not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(ticket)).into_response()
}

pub(crate) async fn list_crm_points_ledgers(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CrmLedgerListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.read") {
        return resp.into_response();
    }

    let (created_from, created_to) =
        match parse_optional_time_range(query.created_from.as_deref(), query.created_to.as_deref())
        {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let customer_id = query
        .customer_id
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let tenant_uuid = uuid::Uuid::from(tenant_id);
    let total = match sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)::bigint
        from crm_points_ledgers
        where tenant_id = $1
          and ($2::text is null
               or customer_name ilike '%' || $2 || '%'
               or coalesce(customer_id, '') ilike '%' || $2 || '%'
               or coalesce(ref_id, '') ilike '%' || $2 || '%')
          and ($3::text is null or customer_id = $3)
          and ($4::timestamptz is null or created_at >= $4)
          and ($5::timestamptz is null or created_at <= $5)
        "#,
    )
    .bind(tenant_uuid)
    .bind(keyword.clone())
    .bind(customer_id.clone())
    .bind(created_from)
    .bind(created_to)
    .fetch_one(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items: Vec<CrmPointLedger> = match sqlx::query_as(
        r#"
        select
          ledger_id,
          customer_id,
          customer_name,
          change_points,
          balance_points,
          reason,
          ref_type,
          ref_id,
          created_at
        from crm_points_ledgers
        where tenant_id = $1
          and ($2::text is null
               or customer_name ilike '%' || $2 || '%'
               or coalesce(customer_id, '') ilike '%' || $2 || '%'
               or coalesce(ref_id, '') ilike '%' || $2 || '%')
          and ($3::text is null or customer_id = $3)
          and ($4::timestamptz is null or created_at >= $4)
          and ($5::timestamptz is null or created_at <= $5)
        order by created_at desc, ledger_id desc
        offset $6 limit $7
        "#,
    )
    .bind(tenant_uuid)
    .bind(keyword)
    .bind(customer_id)
    .bind(created_from)
    .bind(created_to)
    .bind(offset)
    .bind(limit)
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(CrmPointLedgerListResponse { items, total }),
    )
        .into_response()
}

pub(crate) async fn list_crm_stored_value_ledgers(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CrmLedgerListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "customers.read") {
        return resp.into_response();
    }

    let (created_from, created_to) =
        match parse_optional_time_range(query.created_from.as_deref(), query.created_to.as_deref())
        {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let customer_id = query
        .customer_id
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let tenant_uuid = uuid::Uuid::from(tenant_id);
    let total = match sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)::bigint
        from crm_stored_value_ledgers
        where tenant_id = $1
          and ($2::text is null
               or customer_name ilike '%' || $2 || '%'
               or coalesce(customer_id, '') ilike '%' || $2 || '%'
               or coalesce(ref_id, '') ilike '%' || $2 || '%')
          and ($3::text is null or customer_id = $3)
          and ($4::timestamptz is null or created_at >= $4)
          and ($5::timestamptz is null or created_at <= $5)
        "#,
    )
    .bind(tenant_uuid)
    .bind(keyword.clone())
    .bind(customer_id.clone())
    .bind(created_from)
    .bind(created_to)
    .fetch_one(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items: Vec<CrmStoredValueLedger> = match sqlx::query_as(
        r#"
        select
          ledger_id,
          customer_id,
          customer_name,
          change_cent,
          balance_cent,
          reason,
          ref_type,
          ref_id,
          created_at
        from crm_stored_value_ledgers
        where tenant_id = $1
          and ($2::text is null
               or customer_name ilike '%' || $2 || '%'
               or coalesce(customer_id, '') ilike '%' || $2 || '%'
               or coalesce(ref_id, '') ilike '%' || $2 || '%')
          and ($3::text is null or customer_id = $3)
          and ($4::timestamptz is null or created_at >= $4)
          and ($5::timestamptz is null or created_at <= $5)
        order by created_at desc, ledger_id desc
        offset $6 limit $7
        "#,
    )
    .bind(tenant_uuid)
    .bind(keyword)
    .bind(customer_id)
    .bind(created_from)
    .bind(created_to)
    .bind(offset)
    .bind(limit)
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(CrmStoredValueLedgerListResponse { items, total }),
    )
        .into_response()
}
