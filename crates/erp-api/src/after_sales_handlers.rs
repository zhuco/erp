use super::*;

pub(crate) async fn list_after_sale_cases(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<AfterSaleCaseListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };

    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let mut platform = match query.platform.as_deref() {
        Some(v) => match Platform::from_str(v) {
            Ok(p) => Some(p),
            Err(_) => return bad_request("bad platform").into_response(),
        },
        None => None,
    };
    let shop_id = match query.shop_id.as_deref() {
        Some(v) => match ShopId::from_str(v) {
            Ok(s) => Some(s),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let refund_state = match query.refund_state.as_deref() {
        Some(v) => match RefundState::from_str(v) {
            Ok(s) => Some(s),
            Err(_) => return bad_request("bad refund_state").into_response(),
        },
        None => None,
    };
    let after_sale_state = match query.after_sale_state.as_deref() {
        Some(v) => match AfterSaleState::from_str(v) {
            Ok(s) => Some(s),
            Err(_) => return bad_request("bad after_sale_state").into_response(),
        },
        None => None,
    };

    let pdd_only = match pdd_only_mode_enabled(&state, tenant_id).await {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if pdd_only {
        match platform {
            Some(p) if p != Platform::Pdd => {
                warn!(
                    tenant_id = %tenant_id,
                    platform = %p,
                    "pdd-only mode blocked after-sale list"
                );
                return pdd_only_forbidden().into_response();
            }
            None => {
                platform = Some(Platform::Pdd);
            }
            _ => {}
        }
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let cases = match state
        .after_sale_repo
        .list_cases(
            tenant_id,
            erp_repo::after_sale_cases::AfterSaleCaseListFilters {
                platform,
                shop_id,
                refund_state,
                after_sale_state,
                limit,
                offset,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = AfterSaleCaseListResponse {
        data: cases,
        limit,
        offset,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_after_sale_case(
    State(state): State<AppState>,
    Path((tenant_id, case_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };

    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let case_id = match AfterSaleCaseId::from_str(&case_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad case_id").into_response(),
    };

    let pdd_only = match pdd_only_mode_enabled(&state, tenant_id).await {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let detail = match state
        .after_sale_repo
        .get_case_detail(tenant_id, case_id)
        .await
    {
        Ok(Some(detail)) => detail,
        Ok(None) => return not_found("case not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if pdd_only && detail.case.platform != Platform::Pdd {
        warn!(
            tenant_id = %tenant_id,
            platform = %detail.case.platform,
            case_id = %case_id,
            "pdd-only mode blocked after-sale detail"
        );
        return pdd_only_forbidden().into_response();
    }

    (StatusCode::OK, Json(detail)).into_response()
}

pub(crate) async fn get_after_sale_case_raw(
    State(state): State<AppState>,
    Path((tenant_id, case_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };

    let case_id = match AfterSaleCaseId::from_str(&case_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad case_id").into_response(),
    };

    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read_raw") {
        return resp.into_response();
    }

    let pdd_only = match pdd_only_mode_enabled(&state, tenant_id).await {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    if pdd_only {
        let detail = match state
            .after_sale_repo
            .get_case_detail(tenant_id, case_id)
            .await
        {
            Ok(Some(detail)) => detail,
            Ok(None) => return not_found("case not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

        if detail.case.platform != Platform::Pdd {
            warn!(
                tenant_id = %tenant_id,
                platform = %detail.case.platform,
                case_id = %case_id,
                "pdd-only mode blocked after-sale raw"
            );
            return pdd_only_forbidden().into_response();
        }
    }

    match state.after_sale_repo.get_case_raw(tenant_id, case_id).await {
        Ok(raw) => (StatusCode::OK, Json(raw)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

pub(crate) async fn list_after_sales(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<AfterSalesListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let status = match query.status.as_deref() {
        Some(raw) => match AfterSalesStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad status").into_response(),
        },
        None => None,
    };
    let after_sales_type = match query.after_sales_type.as_deref() {
        Some(raw) => match AfterSalesType::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad type").into_response(),
        },
        None => None,
    };
    let order_id = match query.order_id.as_deref() {
        Some(raw) => match OrderId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad order_id").into_response(),
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
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let should_update_sla_snapshot = query.status.is_none()
        && query.after_sales_type.is_none()
        && query.order_id.is_none()
        && query.created_from.is_none()
        && query.created_to.is_none();

    let repo = AfterSalesRepo::new(state.db.pool());
    let result = match repo
        .list_after_sales(
            tenant_id,
            AfterSalesListFilters {
                status,
                after_sales_type,
                order_id,
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

    let sla_profiles = load_after_sales_sla_profiles(state.db.pool(), tenant_id).await;
    let now = Utc::now();
    let mut overdue_in_page = 0_u64;
    let items = result
        .items
        .iter()
        .map(|sale| {
            let sla = derive_after_sales_sla_view(sale, &sla_profiles, now);
            if matches!(sla.sla_status.as_deref(), Some("OVERDUE")) {
                overdue_in_page += 1;
            }
            after_sales_view_with_sla(sale, sla)
        })
        .collect();

    if should_update_sla_snapshot {
        metrics().set_after_sales_sla_evaluated(result.total.max(0) as u64);
        let overdue_total =
            match count_after_sales_sla_overdue_snapshot(state.db.pool(), tenant_id).await {
                Ok(v) => v.max(0) as u64,
                Err(err) => {
                    warn!(
                        error = %err,
                        tenant_id = %tenant_id,
                        "count after-sales overdue snapshot failed"
                    );
                    overdue_in_page
                }
            };
        metrics().set_after_sales_sla_overdue(overdue_total);
    }

    let response = AfterSalesListResponse {
        items,
        total: result.total,
        limit,
        offset,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_after_sales(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, after_sales_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("after_sales not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let audit_logs = match load_after_sales_audit_logs(
        state.db.pool(),
        tenant_id,
        detail.after_sales.id,
        detail.refund_doc.as_ref().map(|doc| doc.id),
        200,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(after_sales_detail_view(detail, audit_logs)),
    )
        .into_response()
}

pub(crate) async fn create_after_sales(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    if req.items.is_empty() {
        return bad_request("items required").into_response();
    }

    let order_id = match OrderId::from_str(&req.order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    let after_sales_type = match AfterSalesType::from_str(&req.after_sales_type) {
        Ok(v) => v,
        Err(_) => return bad_request("bad type").into_response(),
    };
    let compensation_status = match req.compensation_status.as_deref() {
        Some(raw) => match CompensationStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad compensation_status").into_response(),
        },
        None => None,
    };
    let evidence_status = match req.evidence_status.as_deref() {
        Some(raw) => match EvidenceStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad evidence_status").into_response(),
        },
        None => None,
    };

    if let Some(reason_code) = req.reason_code.as_deref() {
        let reason_repo =
            erp_repo::after_sales_reasons::AfterSalesReasonsRepo::new(state.db.pool());
        match reason_repo.get_reason(tenant_id, reason_code).await {
            Ok(Some(reason)) => {
                if reason.status.to_uppercase() != "ACTIVE" {
                    return conflict("after_sales reason disabled").into_response();
                }
            }
            Ok(None) => return bad_request("after_sales reason not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    }

    let mut items = Vec::with_capacity(req.items.len());
    let mut amount_sum = 0_i64;
    for item in &req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        if item.qty <= 0 {
            return bad_request("qty must be positive").into_response();
        }
        amount_sum = amount_sum.saturating_add(item.refund_cent);
        items.push(AfterSalesItemCreate {
            sku_id,
            qty: item.qty,
            refund_cent: item.refund_cent,
        });
    }

    let amount_cent = req.amount_cent.unwrap_or(amount_sum);
    let repo = AfterSalesRepo::new(state.db.pool());
    let detail = match repo
        .create_after_sales(AfterSalesCreateInput {
            after_sales_id: None,
            tenant_id,
            order_id,
            after_sales_type,
            reason_code: req.reason_code.clone(),
            amount_cent,
            raw_json: serde_json::to_value(&req).ok(),
            compensation_status,
            evidence_status,
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "after_sales_id": detail.after_sales.id,
        "order_id": detail.after_sales.order_id.to_string(),
        "status": detail.after_sales.status.to_string(),
        "type": detail.after_sales.after_sales_type.to_string(),
        "amount_cent": detail.after_sales.amount_cent,
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.CREATE".to_string(),
            entity_type: "after_sales".to_string(),
            entity_id: Some(detail.after_sales.id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let audit_logs = match load_after_sales_audit_logs(
        state.db.pool(),
        tenant_id,
        detail.after_sales.id,
        detail.refund_doc.as_ref().map(|doc| doc.id),
        200,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(after_sales_detail_view(detail, audit_logs)),
    )
        .into_response()
}

pub(crate) async fn approve_after_sales(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let before = match repo.get_after_sales(tenant_id, after_sales_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("after_sales not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let updated = match repo
        .set_status(
            tenant_id,
            after_sales_id,
            AfterSalesStatus::Approved,
            &[AfterSalesStatus::Open],
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return after_sales_repo_error(e),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let before_json = serde_json::json!({ "status": before.status.to_string() });
    let after_json = serde_json::json!({ "status": updated.status.to_string() });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.APPROVE".to_string(),
            entity_type: "after_sales".to_string(),
            entity_id: Some(updated.id.to_string()),
            before_json: Some(before_json),
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let sales_repo = SalesOrdersRepo::new(state.db.pool());
    let source_order_id = updated.order_id.to_string();
    let sales_order = match sales_repo
        .find_by_source_order(tenant_id, &source_order_id)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "閿€鍞崟鑱斿姩鑾峰彇失败");
            None
        }
    };
    if let Some(sales_order) = sales_order {
        let status_flags = append_status_flags(&sales_order.status_flags, &["HAS_AFTER_SALE"]);
        let updated_order = sales_repo
            .update_after_sales_status(
                tenant_id,
                sales_order.sales_order_id,
                SalesOrderAfterSalesUpdate {
                    refund_status: None,
                    return_status: None,
                    status_flags: Some(status_flags.clone()),
                },
            )
            .await;
        match updated_order {
            Ok(Some(updated_order)) => {
                let before_flags = if sales_order.status_flags.is_empty() {
                    None
                } else {
                    Some(sales_order.status_flags.join(","))
                };
                let after_flags = if updated_order.status_flags.is_empty() {
                    None
                } else {
                    Some(updated_order.status_flags.join(","))
                };
                let from_status =
                    format_sales_order_status("STATUS_FLAGS", before_flags.as_deref());
                let to_status = format_sales_order_status("STATUS_FLAGS", after_flags.as_deref());
                record_sales_order_state_logs(
                    &state,
                    &req_ctx,
                    tenant_id,
                    updated_order.sales_order_id,
                    "SALES_ORDER.AFTERSALES_APPROVED",
                    from_status,
                    to_status,
                    Some(ctx.user_id),
                    None,
                    serde_json::json!({
                        "after_sales_id": updated.id,
                        "after_sales_status": updated.status.to_string(),
                        "source_order_id": source_order_id,
                    }),
                    serde_json::json!({
                        "refund_status": updated_order.refund_status,
                        "return_status": updated_order.return_status,
                        "status_flags": updated_order.status_flags,
                    }),
                )
                .await;
            }
            Ok(None) => {}
            Err(e) => {
                warn!(error = %e, "閿€鍞崟鑱斿姩更新失败");
            }
        }
    }

    (StatusCode::OK, Json(after_sales_view(&updated))).into_response()
}

pub(crate) async fn reject_after_sales(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let before = match repo.get_after_sales(tenant_id, after_sales_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("after_sales not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let updated = match repo
        .set_status(
            tenant_id,
            after_sales_id,
            AfterSalesStatus::Rejected,
            &[AfterSalesStatus::Open, AfterSalesStatus::Approved],
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return after_sales_repo_error(e),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let before_json = serde_json::json!({ "status": before.status.to_string() });
    let after_json = serde_json::json!({ "status": updated.status.to_string() });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.REJECT".to_string(),
            entity_type: "after_sales".to_string(),
            entity_id: Some(updated.id.to_string()),
            before_json: Some(before_json),
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let sales_repo = SalesOrdersRepo::new(state.db.pool());
    let source_order_id = updated.order_id.to_string();
    let sales_order = match sales_repo
        .find_by_source_order(tenant_id, &source_order_id)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "閿€鍞崟鑱斿姩鑾峰彇失败");
            None
        }
    };
    if let Some(sales_order) = sales_order {
        let status_flags = append_status_flags(&sales_order.status_flags, &["HAS_AFTER_SALE"]);
        let updated_order = sales_repo
            .update_after_sales_status(
                tenant_id,
                sales_order.sales_order_id,
                SalesOrderAfterSalesUpdate {
                    refund_status: None,
                    return_status: None,
                    status_flags: Some(status_flags.clone()),
                },
            )
            .await;
        match updated_order {
            Ok(Some(updated_order)) => {
                let before_flags = if sales_order.status_flags.is_empty() {
                    None
                } else {
                    Some(sales_order.status_flags.join(","))
                };
                let after_flags = if updated_order.status_flags.is_empty() {
                    None
                } else {
                    Some(updated_order.status_flags.join(","))
                };
                let from_status =
                    format_sales_order_status("STATUS_FLAGS", before_flags.as_deref());
                let to_status = format_sales_order_status("STATUS_FLAGS", after_flags.as_deref());
                record_sales_order_state_logs(
                    &state,
                    &req_ctx,
                    tenant_id,
                    updated_order.sales_order_id,
                    "SALES_ORDER.AFTERSALES_REJECTED",
                    from_status,
                    to_status,
                    Some(ctx.user_id),
                    None,
                    serde_json::json!({
                        "after_sales_id": updated.id,
                        "after_sales_status": updated.status.to_string(),
                        "source_order_id": source_order_id,
                    }),
                    serde_json::json!({
                        "refund_status": updated_order.refund_status,
                        "return_status": updated_order.return_status,
                        "status_flags": updated_order.status_flags,
                    }),
                )
                .await;
            }
            Ok(None) => {}
            Err(e) => {
                warn!(error = %e, "閿€鍞崟鑱斿姩更新失败");
            }
        }
    }

    (StatusCode::OK, Json(after_sales_view(&updated))).into_response()
}

pub(crate) async fn receive_after_sales(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<AfterSalesReceiveRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let putaway_done = req.putaway_done.unwrap_or(true);

    let settings_repo = SettingsRepo::new(state.db.pool());
    let link_inbound = match settings_repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some("TENANT".to_string()),
                scope_id: None,
                key: Some("service.enable_return_inbound_link".to_string()),
            },
        )
        .await
    {
        Ok(settings) => settings
            .first()
            .and_then(|setting| parse_bool_setting(&setting.value_json))
            .unwrap_or(true),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let detail = match repo
        .receive_return(ReturnReceiptCreateInput {
            tenant_id,
            after_sales_id,
            warehouse_id,
            putaway_done,
            received_at: Some(Utc::now()),
            link_inbound,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => {
            let audit_repo = AuditLogsRepo::new(state.db.pool());
            let after_json = serde_json::json!({
                "after_sales_id": after_sales_id,
                "warehouse_id": warehouse_id,
                "error": e.to_string(),
            });
            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id,
                    user_id: Some(ctx.user_id),
                    action: "AFTERSALES.RECEIVE_FAILED".to_string(),
                    entity_type: "after_sales".to_string(),
                    entity_id: Some(after_sales_id.to_string()),
                    before_json: None,
                    after_json: Some(after_json),
                    ip: None,
                    ua: None,
                })
                .await;
            return after_sales_repo_error(e);
        }
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "after_sales_id": detail.after_sales.id,
        "status": detail.after_sales.status.to_string(),
        "warehouse_id": warehouse_id,
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.RECEIVE".to_string(),
            entity_type: "after_sales".to_string(),
            entity_id: Some(detail.after_sales.id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let sales_repo = SalesOrdersRepo::new(state.db.pool());
    let source_order_id = detail.after_sales.order_id.to_string();
    let sales_order = match sales_repo
        .find_by_source_order(tenant_id, &source_order_id)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "閿€鍞崟鑱斿姩鑾峰彇失败");
            None
        }
    };
    if let Some(sales_order) = sales_order {
        let status_flags = append_status_flags(&sales_order.status_flags, &["HAS_AFTER_SALE"]);
        let updated_order = sales_repo
            .update_after_sales_status(
                tenant_id,
                sales_order.sales_order_id,
                SalesOrderAfterSalesUpdate {
                    refund_status: None,
                    return_status: Some("RETURNED".to_string()),
                    status_flags: Some(status_flags.clone()),
                },
            )
            .await;
        match updated_order {
            Ok(Some(updated_order)) => {
                let from_status = format_sales_order_status(
                    "RETURN_STATUS",
                    sales_order.return_status.as_deref(),
                );
                let to_status = format_sales_order_status(
                    "RETURN_STATUS",
                    updated_order.return_status.as_deref(),
                );
                record_sales_order_state_logs(
                    &state,
                    &req_ctx,
                    tenant_id,
                    updated_order.sales_order_id,
                    "SALES_ORDER.AFTERSALES_RECEIVED",
                    from_status,
                    to_status,
                    Some(ctx.user_id),
                    None,
                    serde_json::json!({
                        "after_sales_id": detail.after_sales.id,
                        "after_sales_status": detail.after_sales.status.to_string(),
                        "warehouse_id": warehouse_id,
                        "source_order_id": source_order_id,
                    }),
                    serde_json::json!({
                        "refund_status": updated_order.refund_status,
                        "return_status": updated_order.return_status,
                        "status_flags": updated_order.status_flags,
                    }),
                )
                .await;
            }
            Ok(None) => {}
            Err(e) => {
                warn!(error = %e, "閿€鍞崟鑱斿姩更新失败");
            }
        }
    }

    let audit_logs = match load_after_sales_audit_logs(
        state.db.pool(),
        tenant_id,
        detail.after_sales.id,
        detail.refund_doc.as_ref().map(|doc| doc.id),
        200,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(after_sales_detail_view(detail, audit_logs)),
    )
        .into_response()
}

pub(crate) async fn putaway_after_sales(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let detail = match repo.putaway_return_receipt(tenant_id, after_sales_id).await {
        Ok(v) => v,
        Err(e) => {
            let audit_repo = AuditLogsRepo::new(state.db.pool());
            let after_json = serde_json::json!({
                "after_sales_id": after_sales_id,
                "error": e.to_string(),
            });
            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id,
                    user_id: Some(ctx.user_id),
                    action: "AFTERSALES.PUTAWAY_FAILED".to_string(),
                    entity_type: "after_sales".to_string(),
                    entity_id: Some(after_sales_id.to_string()),
                    before_json: None,
                    after_json: Some(after_json),
                    ip: None,
                    ua: None,
                })
                .await;
            return after_sales_repo_error(e);
        }
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "after_sales_id": detail.after_sales.id,
        "status": detail.after_sales.status.to_string(),
        "warehouse_id": detail.return_receipt.as_ref().map(|receipt| receipt.warehouse_id),
        "return_receipt_status": detail
            .return_receipt
            .as_ref()
            .map(|receipt| receipt.status.to_string()),
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.PUTAWAY".to_string(),
            entity_type: "after_sales".to_string(),
            entity_id: Some(detail.after_sales.id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let audit_logs = match load_after_sales_audit_logs(
        state.db.pool(),
        tenant_id,
        detail.after_sales.id,
        detail.refund_doc.as_ref().map(|doc| doc.id),
        200,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(after_sales_detail_view(detail, audit_logs)),
    )
        .into_response()
}

pub(crate) async fn cancel_after_sales_receive(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let detail = match repo.cancel_return_receipt(tenant_id, after_sales_id).await {
        Ok(v) => v,
        Err(e) => {
            let audit_repo = AuditLogsRepo::new(state.db.pool());
            let after_json = serde_json::json!({
                "after_sales_id": after_sales_id,
                "error": e.to_string(),
            });
            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id,
                    user_id: Some(ctx.user_id),
                    action: "AFTERSALES.RECEIVE_CANCEL_FAILED".to_string(),
                    entity_type: "after_sales".to_string(),
                    entity_id: Some(after_sales_id.to_string()),
                    before_json: None,
                    after_json: Some(after_json),
                    ip: None,
                    ua: None,
                })
                .await;
            return after_sales_repo_error(e);
        }
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "after_sales_id": detail.after_sales.id,
        "status": detail.after_sales.status.to_string(),
        "receipt_status": detail.return_receipt.as_ref().map(|r| r.status.to_string()),
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.RECEIVE_CANCEL".to_string(),
            entity_type: "after_sales".to_string(),
            entity_id: Some(detail.after_sales.id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let audit_logs = match load_after_sales_audit_logs(
        state.db.pool(),
        tenant_id,
        detail.after_sales.id,
        detail.refund_doc.as_ref().map(|doc| doc.id),
        200,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(after_sales_detail_view(detail, audit_logs)),
    )
        .into_response()
}

pub(crate) async fn redo_after_sales_receive(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesReceiveRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let putaway_done = req.putaway_done.unwrap_or(true);

    let settings_repo = SettingsRepo::new(state.db.pool());
    let link_inbound = match settings_repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some("TENANT".to_string()),
                scope_id: None,
                key: Some("service.enable_return_inbound_link".to_string()),
            },
        )
        .await
    {
        Ok(settings) => settings
            .first()
            .and_then(|setting| parse_bool_setting(&setting.value_json))
            .unwrap_or(true),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let detail = match repo
        .redo_return_receipt(ReturnReceiptCreateInput {
            tenant_id,
            after_sales_id,
            warehouse_id,
            putaway_done,
            received_at: Some(Utc::now()),
            link_inbound,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => {
            let audit_repo = AuditLogsRepo::new(state.db.pool());
            let after_json = serde_json::json!({
                "after_sales_id": after_sales_id,
                "warehouse_id": warehouse_id,
                "error": e.to_string(),
            });
            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id,
                    user_id: Some(ctx.user_id),
                    action: "AFTERSALES.RECEIVE_REDO_FAILED".to_string(),
                    entity_type: "after_sales".to_string(),
                    entity_id: Some(after_sales_id.to_string()),
                    before_json: None,
                    after_json: Some(after_json),
                    ip: None,
                    ua: None,
                })
                .await;
            return after_sales_repo_error(e);
        }
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "after_sales_id": detail.after_sales.id,
        "status": detail.after_sales.status.to_string(),
        "warehouse_id": warehouse_id,
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.RECEIVE_REDO".to_string(),
            entity_type: "after_sales".to_string(),
            entity_id: Some(detail.after_sales.id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let audit_logs = match load_after_sales_audit_logs(
        state.db.pool(),
        tenant_id,
        detail.after_sales.id,
        detail.refund_doc.as_ref().map(|doc| doc.id),
        200,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(after_sales_detail_view(detail, audit_logs)),
    )
        .into_response()
}

pub(crate) async fn request_after_sales_refund(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<AfterSalesRefundRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, after_sales_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("after_sales not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if matches!(
        detail.after_sales.status,
        AfterSalesStatus::Rejected | AfterSalesStatus::Closed | AfterSalesStatus::Refunded
    ) {
        return conflict("after_sales status not refundable").into_response();
    }

    let amount_cent = req.amount_cent.unwrap_or(detail.after_sales.amount_cent);
    let channel = match req.channel.as_deref() {
        Some(raw) => match RefundChannel::from_str(raw) {
            Ok(v) => v,
            Err(_) => return bad_request("bad refund channel").into_response(),
        },
        None => RefundChannel::Manual,
    };

    let (refund_doc, should_enqueue) = match repo
        .upsert_refund_doc(RefundDocCreateInput {
            tenant_id,
            after_sales_id,
            amount_cent,
            channel,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if should_enqueue {
        let payload = serde_json::to_value(RefundRequestedPayload {
            tenant_id,
            after_sales_id,
            refund_doc_id: refund_doc.id,
            amount_cent: refund_doc.amount_cent,
            channel: refund_doc.channel,
            after_sales_type: detail.after_sales.after_sales_type,
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
        let payload = match payload {
            Ok(v) => v,
            Err(resp) => return resp,
        };

        let outbox_repo = OutboxRepo::new(state.db.pool());
        if let Err(e) = outbox_repo
            .enqueue(tenant_id, REFUND_REQUESTED_EVENT, &payload)
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let audit_exists = match audit_repo
        .list_logs(
            tenant_id,
            AuditLogListFilters {
                action: Some("AFTERSALES.REFUND_REQUESTED".to_string()),
                entity_type: Some("refund_doc".to_string()),
                entity_id: Some(refund_doc.id.to_string()),
                user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit: 1,
            },
        )
        .await
    {
        Ok(v) => !v.items.is_empty(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    if !audit_exists {
        let after_json = serde_json::json!({
            "refund_doc_id": refund_doc.id,
            "after_sales_id": refund_doc.after_sales_id,
            "amount_cent": refund_doc.amount_cent,
            "status": refund_doc.status.to_string(),
            "channel": refund_doc.channel.to_string(),
        });
        if let Err(err) = audit_repo
            .insert(AuditLogCreate {
                tenant_id,
                user_id: Some(ctx.user_id),
                action: "AFTERSALES.REFUND_REQUESTED".to_string(),
                entity_type: "refund_doc".to_string(),
                entity_id: Some(refund_doc.id.to_string()),
                before_json: None,
                after_json: Some(after_json),
                ip: None,
                ua: None,
            })
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
    }

    let sales_repo = SalesOrdersRepo::new(state.db.pool());
    let source_order_id = detail.after_sales.order_id.to_string();
    let sales_order = match sales_repo
        .find_by_source_order(tenant_id, &source_order_id)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "sales-order linkage lookup failed");
            None
        }
    };
    if let Some(sales_order) = sales_order {
        let status_flags = append_status_flags(&sales_order.status_flags, &["HAS_AFTER_SALE"]);
        let need_update = sales_order.refund_status != "PARTIAL_REFUND"
            || sales_order.status_flags != status_flags;
        if need_update {
            let updated_order = sales_repo
                .update_after_sales_status(
                    tenant_id,
                    sales_order.sales_order_id,
                    SalesOrderAfterSalesUpdate {
                        refund_status: Some("PARTIAL_REFUND".to_string()),
                        return_status: None,
                        status_flags: Some(status_flags.clone()),
                    },
                )
                .await;
            match updated_order {
                Ok(Some(updated_order)) => {
                    let from_status = format_sales_order_status(
                        "REFUND_STATUS",
                        Some(&sales_order.refund_status),
                    );
                    let to_status = format_sales_order_status(
                        "REFUND_STATUS",
                        Some(&updated_order.refund_status),
                    );
                    record_sales_order_state_logs(
                        &state,
                        &req_ctx,
                        tenant_id,
                        updated_order.sales_order_id,
                        "SALES_ORDER.AFTERSALES_REFUND_REQUESTED",
                        from_status,
                        to_status,
                        Some(ctx.user_id),
                        None,
                        serde_json::json!({
                            "after_sales_id": detail.after_sales.id,
                            "after_sales_status": detail.after_sales.status.to_string(),
                            "refund_doc_id": refund_doc.id,
                            "amount_cent": refund_doc.amount_cent,
                            "channel": refund_doc.channel.to_string(),
                            "source_order_id": source_order_id,
                        }),
                        serde_json::json!({
                            "refund_status": updated_order.refund_status,
                            "return_status": updated_order.return_status,
                            "status_flags": updated_order.status_flags,
                        }),
                    )
                    .await;
                }
                Ok(None) => {}
                Err(e) => {
                    warn!(error = %e, "sales-order linkage update failed");
                }
            }
        }
    }

    (StatusCode::OK, Json(refund_doc_view(&refund_doc))).into_response()
}

pub(crate) async fn get_after_sales_raw(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read_raw") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let record = match repo.get_after_sales(tenant_id, after_sales_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("after_sales not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(record.raw_json)).into_response()
}

pub(crate) async fn list_after_sales_logs(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Query(query): Query<AfterSalesAuditLogQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };
    let limit = query.limit.unwrap_or(200).clamp(1, 200);

    let repo = AfterSalesRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, after_sales_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("after_sales not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs = match load_after_sales_audit_logs(
        state.db.pool(),
        tenant_id,
        detail.after_sales.id,
        detail.refund_doc.as_ref().map(|doc| doc.id),
        limit,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = logs
        .into_iter()
        .map(|log| AfterSalesAuditLogView {
            audit_id: log.audit_id,
            action: log.action,
            user_id: log.user_id.map(uuid::Uuid::from),
            before_json: log.before_json,
            after_json: log.after_json,
            created_at: log.created_at,
        })
        .collect::<Vec<_>>();

    (StatusCode::OK, Json(items)).into_response()
}

pub(crate) async fn list_after_sales_reasons(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<AfterSalesReasonListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = erp_repo::after_sales_reasons::AfterSalesReasonsRepo::new(state.db.pool());
    let result = match repo
        .list_reasons(
            tenant_id,
            erp_repo::after_sales_reasons::AfterSalesReasonListFilters {
                category: query.category.clone(),
                status: query.status.clone(),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result
        .items
        .into_iter()
        .map(|reason| AfterSalesReasonView {
            reason_code: reason.reason_code,
            label: reason.label,
            category: reason.category,
            status: reason.status,
            sort_order: reason.sort_order,
            created_at: reason.created_at,
            updated_at: reason.updated_at,
        })
        .collect();

    (
        StatusCode::OK,
        Json(AfterSalesReasonListResponse {
            items,
            total: result.total,
            limit,
            offset,
        }),
    )
        .into_response()
}

pub(crate) async fn create_after_sales_reason(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesReasonRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let reason_code = match req.reason_code.as_deref() {
        Some(code) if !code.trim().is_empty() => code.trim().to_string(),
        _ => return bad_request("reason_code required").into_response(),
    };
    let status = normalize_directory_status(req.status.as_deref());
    let sort_order = req.sort_order.unwrap_or(0);

    let repo = erp_repo::after_sales_reasons::AfterSalesReasonsRepo::new(state.db.pool());
    let before = match repo.get_reason(tenant_id, &reason_code).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let reason = match repo
        .upsert_reason(erp_repo::after_sales_reasons::AfterSalesReasonUpsert {
            tenant_id,
            reason_code: reason_code.clone(),
            label: req.label.clone(),
            category: req.category.clone(),
            status: status.clone(),
            sort_order,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "reason_code": reason.reason_code,
        "label": reason.label,
        "category": reason.category,
        "status": reason.status,
        "sort_order": reason.sort_order,
    });
    let action = if before.is_some() {
        "AFTERSALES.REASON_UPDATED"
    } else {
        "AFTERSALES.REASON_CREATED"
    };
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: action.to_string(),
            entity_type: "after_sales_reason".to_string(),
            entity_id: Some(reason.reason_code.clone()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    (
        StatusCode::OK,
        Json(AfterSalesReasonView {
            reason_code: reason.reason_code,
            label: reason.label,
            category: reason.category,
            status: reason.status,
            sort_order: reason.sort_order,
            created_at: reason.created_at,
            updated_at: reason.updated_at,
        }),
    )
        .into_response()
}

pub(crate) async fn update_after_sales_reason(
    State(state): State<AppState>,
    Path((tenant_id, reason_code)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesReasonRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let reason_code = reason_code.trim().to_string();
    if reason_code.is_empty() {
        return bad_request("reason_code required").into_response();
    }

    let status = normalize_directory_status(req.status.as_deref());
    let sort_order = req.sort_order.unwrap_or(0);

    let repo = erp_repo::after_sales_reasons::AfterSalesReasonsRepo::new(state.db.pool());
    let reason = match repo
        .upsert_reason(erp_repo::after_sales_reasons::AfterSalesReasonUpsert {
            tenant_id,
            reason_code: reason_code.clone(),
            label: req.label.clone(),
            category: req.category.clone(),
            status: status.clone(),
            sort_order,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "reason_code": reason.reason_code,
        "label": reason.label,
        "category": reason.category,
        "status": reason.status,
        "sort_order": reason.sort_order,
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.REASON_UPDATED".to_string(),
            entity_type: "after_sales_reason".to_string(),
            entity_id: Some(reason.reason_code.clone()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    (
        StatusCode::OK,
        Json(AfterSalesReasonView {
            reason_code: reason.reason_code,
            label: reason.label,
            category: reason.category,
            status: reason.status,
            sort_order: reason.sort_order,
            created_at: reason.created_at,
            updated_at: reason.updated_at,
        }),
    )
        .into_response()
}

pub(crate) async fn delete_after_sales_reason(
    State(state): State<AppState>,
    Path((tenant_id, reason_code)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let reason_code = reason_code.trim();
    if reason_code.is_empty() {
        return bad_request("reason_code required").into_response();
    }

    let repo = erp_repo::after_sales_reasons::AfterSalesReasonsRepo::new(state.db.pool());
    if let Err(e) = repo.delete_reason(tenant_id, reason_code).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "reason_code": reason_code,
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.REASON_DELETED".to_string(),
            entity_type: "after_sales_reason".to_string(),
            entity_id: Some(reason_code.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    (StatusCode::NO_CONTENT, Json(serde_json::json!({}))).into_response()
}

pub(crate) async fn list_after_sales_quality_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<AfterSalesQualityRuleListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let after_sales_type = match query.after_sales_type.as_deref() {
        Some(raw) => match AfterSalesType::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad after_sales_type").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo =
        erp_repo::after_sales_quality_rules::AfterSalesQualityRulesRepo::new(state.db.pool());
    let result = match repo
        .list_rules(
            tenant_id,
            erp_repo::after_sales_quality_rules::AfterSalesQualityRuleListFilters {
                after_sales_type,
                status: query.status.clone(),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result
        .items
        .into_iter()
        .map(|rule| AfterSalesQualityRuleView {
            rule_id: rule.rule_id,
            after_sales_type: rule.after_sales_type.to_string(),
            result_code: rule.result_code,
            status: rule.status,
            sort_order: rule.sort_order,
            created_at: rule.created_at,
            updated_at: rule.updated_at,
        })
        .collect();

    (
        StatusCode::OK,
        Json(AfterSalesQualityRuleListResponse {
            items,
            total: result.total,
            limit,
            offset,
        }),
    )
        .into_response()
}

pub(crate) async fn create_after_sales_quality_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesQualityRuleRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let after_sales_type = match AfterSalesType::from_str(&req.after_sales_type) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_type").into_response(),
    };
    let result_code = match normalize_quality_result_code(&req.result_code) {
        Some(v) => v,
        None => return bad_request("invalid result_code").into_response(),
    };
    let status = normalize_directory_status(req.status.as_deref());
    let sort_order = req.sort_order.unwrap_or(0);

    let repo =
        erp_repo::after_sales_quality_rules::AfterSalesQualityRulesRepo::new(state.db.pool());
    let rule = match repo
        .create_rule(
            erp_repo::after_sales_quality_rules::AfterSalesQualityRuleCreate {
                tenant_id,
                after_sales_type,
                result_code: result_code.clone(),
                status: status.clone(),
                sort_order,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "rule_id": rule.rule_id,
        "after_sales_type": rule.after_sales_type.to_string(),
        "result_code": rule.result_code,
        "status": rule.status,
        "sort_order": rule.sort_order,
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.QUALITY_RULE_CREATED".to_string(),
            entity_type: "after_sales_quality_rule".to_string(),
            entity_id: Some(rule.rule_id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    (
        StatusCode::OK,
        Json(AfterSalesQualityRuleView {
            rule_id: rule.rule_id,
            after_sales_type: rule.after_sales_type.to_string(),
            result_code: rule.result_code,
            status: rule.status,
            sort_order: rule.sort_order,
            created_at: rule.created_at,
            updated_at: rule.updated_at,
        }),
    )
        .into_response()
}

pub(crate) async fn update_after_sales_quality_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesQualityRuleRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    let after_sales_type = match AfterSalesType::from_str(&req.after_sales_type) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_type").into_response(),
    };
    let result_code = match normalize_quality_result_code(&req.result_code) {
        Some(v) => v,
        None => return bad_request("invalid result_code").into_response(),
    };
    let status = normalize_directory_status(req.status.as_deref());
    let sort_order = req.sort_order.unwrap_or(0);

    let repo =
        erp_repo::after_sales_quality_rules::AfterSalesQualityRulesRepo::new(state.db.pool());
    let rule = match repo
        .update_rule(
            erp_repo::after_sales_quality_rules::AfterSalesQualityRuleUpdate {
                tenant_id,
                rule_id,
                after_sales_type,
                result_code: result_code.clone(),
                status: status.clone(),
                sort_order,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "rule_id": rule.rule_id,
        "after_sales_type": rule.after_sales_type.to_string(),
        "result_code": rule.result_code,
        "status": rule.status,
        "sort_order": rule.sort_order,
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.QUALITY_RULE_UPDATED".to_string(),
            entity_type: "after_sales_quality_rule".to_string(),
            entity_id: Some(rule.rule_id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    (
        StatusCode::OK,
        Json(AfterSalesQualityRuleView {
            rule_id: rule.rule_id,
            after_sales_type: rule.after_sales_type.to_string(),
            result_code: rule.result_code,
            status: rule.status,
            sort_order: rule.sort_order,
            created_at: rule.created_at,
            updated_at: rule.updated_at,
        }),
    )
        .into_response()
}

pub(crate) async fn delete_after_sales_quality_rule(
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
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };

    let repo =
        erp_repo::after_sales_quality_rules::AfterSalesQualityRulesRepo::new(state.db.pool());
    if let Err(e) = repo.delete_rule(tenant_id, rule_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "rule_id": rule_id,
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.QUALITY_RULE_DELETED".to_string(),
            entity_type: "after_sales_quality_rule".to_string(),
            entity_id: Some(rule_id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    (StatusCode::NO_CONTENT, Json(serde_json::json!({}))).into_response()
}

pub(crate) async fn create_after_sales_exchange_shipment(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesShipmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };
    let origin_shipment_id = match req.origin_shipment_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad origin_shipment_id").into_response(),
        },
        None => None,
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, after_sales_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("after_sales not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if detail.after_sales.after_sales_type != AfterSalesType::Exchange {
        return conflict("after_sales type not exchange").into_response();
    }
    if !matches!(
        detail.after_sales.status,
        AfterSalesStatus::Approved | AfterSalesStatus::Received
    ) {
        return conflict("after_sales status not shippable").into_response();
    }

    let inventory_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !inventory_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let fulfillment_repo = FulfillmentRepo::new(state.db.pool());
    let result = match fulfillment_repo
        .create_exchange_shipments(
            tenant_id,
            detail.after_sales.order_id,
            req.carrier_code,
            origin_shipment_id,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let outbound_items: Vec<OutboundDocItemInput> = detail
        .items
        .iter()
        .filter(|item| item.qty > 0)
        .map(|item| OutboundDocItemInput {
            sku_id: item.sku_id,
            planned_qty: item.qty,
        })
        .collect();
    if outbound_items.is_empty() {
        return conflict("after_sales items empty").into_response();
    }

    let wms_repo = WmsRepo::new(state.db.pool());
    let mut exchange_docs = Vec::new();
    for shipment in &result.shipments {
        let doc_no = format!("XCHG-{}", shipment.shipment_id);
        let outbound = match wms_repo
            .create_outbound_doc(OutboundDocCreateInput {
                tenant_id,
                warehouse_id: shipment.warehouse_id,
                doc_type: "EXCHANGE".to_string(),
                shipment_id: Some(shipment.shipment_id),
                doc_no,
                items: outbound_items.clone(),
            })
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        exchange_docs.push(ExchangeDocCreateInput {
            outbound_doc_id: Some(outbound.doc.outbound_doc_id),
            shipment_id: Some(shipment.shipment_id),
        });
    }
    if let Err(e) = repo
        .create_exchange_docs(tenant_id, after_sales_id, &exchange_docs)
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let shipment_records = result
        .shipments
        .iter()
        .map(|shipment| AfterSalesShipmentCreate {
            shipment_id: shipment.shipment_id,
            shipment_type: "EXCHANGE".to_string(),
            origin_shipment_id,
        })
        .collect::<Vec<_>>();
    if let Err(e) = repo
        .record_shipments(tenant_id, after_sales_id, &shipment_records)
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let shortages = result
        .shortages
        .into_iter()
        .map(|shortage| OmsAllocationShortageView {
            order_item_id: shortage.order_item_id,
            sku_id: shortage.sku_id,
            requested: shortage.requested,
            available: shortage.available,
        })
        .collect::<Vec<_>>();

    let mut shipments = Vec::new();
    for shipment in result.shipments {
        let items =
            match load_oms_shipment_items(state.db.pool(), tenant_id, shipment.shipment_id).await {
                Ok(v) => v,
                Err(resp) => return resp,
            };
        shipments.push(oms_shipment_summary(
            uuid::Uuid::from(tenant_id),
            shipment,
            Some(items),
        ));
    }

    let status = if shortages.is_empty() {
        "ALLOCATED"
    } else {
        "SHORTAGE"
    };

    let mut after_sales_status = detail.after_sales.status;
    if shortages.is_empty() {
        after_sales_status = match repo
            .set_status(
                tenant_id,
                after_sales_id,
                AfterSalesStatus::Closed,
                &[AfterSalesStatus::Approved, AfterSalesStatus::Received],
            )
            .await
        {
            Ok(v) => v.status,
            Err(e) => return after_sales_repo_error(e),
        };
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "after_sales_id": detail.after_sales.id,
        "order_id": detail.after_sales.order_id.to_string(),
        "shipment_ids": shipments.iter().map(|s| s.shipment_id).collect::<Vec<_>>(),
        "outbound_doc_ids": exchange_docs
            .iter()
            .filter_map(|doc| doc.outbound_doc_id)
            .collect::<Vec<_>>(),
        "origin_shipment_id": origin_shipment_id,
        "shortage_count": shortages.len(),
        "shipment_type": "EXCHANGE",
        "after_sales_status": after_sales_status.to_string(),
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.EXCHANGE_SHIPMENT_CREATED".to_string(),
            entity_type: "after_sales".to_string(),
            entity_id: Some(detail.after_sales.id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    if let Err(err) = refresh_profit_snapshot_for_order(
        &state,
        tenant_id,
        detail.after_sales.order_id,
        Some(ctx.user_id),
        "AFTERSALES.EXCHANGE_SHIPMENT_CREATED",
    )
    .await
    {
        warn!(
            tenant_id = %tenant_id,
            after_sales_id = %after_sales_id,
            order_id = %detail.after_sales.order_id,
            error = %err,
            "failed to refresh profit snapshot after after-sales exchange shipment"
        );
    }

    let response = OmsAllocationResponse {
        status: status.to_string(),
        shipments,
        shortages,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_after_sales_resend_shipment(
    State(state): State<AppState>,
    Path((tenant_id, after_sales_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesShipmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };
    let origin_shipment_id = match req.origin_shipment_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad origin_shipment_id").into_response(),
        },
        None => None,
    };

    let repo = AfterSalesRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, after_sales_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("after_sales not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if detail.after_sales.after_sales_type != AfterSalesType::Resend {
        return conflict("after_sales type not resend").into_response();
    }
    if !matches!(
        detail.after_sales.status,
        AfterSalesStatus::Approved | AfterSalesStatus::Received
    ) {
        return conflict("after_sales status not shippable").into_response();
    }

    let inventory_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !inventory_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let fulfillment_repo = FulfillmentRepo::new(state.db.pool());
    let result = match fulfillment_repo
        .create_resend_shipments(
            tenant_id,
            detail.after_sales.order_id,
            req.carrier_code,
            origin_shipment_id,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shipment_records = result
        .shipments
        .iter()
        .map(|shipment| AfterSalesShipmentCreate {
            shipment_id: shipment.shipment_id,
            shipment_type: "RESEND".to_string(),
            origin_shipment_id,
        })
        .collect::<Vec<_>>();
    if let Err(e) = repo
        .record_shipments(tenant_id, after_sales_id, &shipment_records)
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let shortages = result
        .shortages
        .into_iter()
        .map(|shortage| OmsAllocationShortageView {
            order_item_id: shortage.order_item_id,
            sku_id: shortage.sku_id,
            requested: shortage.requested,
            available: shortage.available,
        })
        .collect::<Vec<_>>();

    let mut shipments = Vec::new();
    for shipment in result.shipments {
        let items =
            match load_oms_shipment_items(state.db.pool(), tenant_id, shipment.shipment_id).await {
                Ok(v) => v,
                Err(resp) => return resp,
            };
        shipments.push(oms_shipment_summary(
            uuid::Uuid::from(tenant_id),
            shipment,
            Some(items),
        ));
    }

    let status = if shortages.is_empty() {
        "ALLOCATED"
    } else {
        "SHORTAGE"
    };

    let mut after_sales_status = detail.after_sales.status;
    if shortages.is_empty() {
        after_sales_status = match repo
            .set_status(
                tenant_id,
                after_sales_id,
                AfterSalesStatus::Closed,
                &[AfterSalesStatus::Approved, AfterSalesStatus::Received],
            )
            .await
        {
            Ok(v) => v.status,
            Err(e) => return after_sales_repo_error(e),
        };
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::json!({
        "after_sales_id": detail.after_sales.id,
        "order_id": detail.after_sales.order_id.to_string(),
        "shipment_ids": shipments.iter().map(|s| s.shipment_id).collect::<Vec<_>>(),
        "origin_shipment_id": origin_shipment_id,
        "shortage_count": shortages.len(),
        "shipment_type": "RESEND",
        "after_sales_status": after_sales_status.to_string(),
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "AFTERSALES.RESEND_SHIPMENT_CREATED".to_string(),
            entity_type: "after_sales".to_string(),
            entity_id: Some(detail.after_sales.id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    if let Err(err) = refresh_profit_snapshot_for_order(
        &state,
        tenant_id,
        detail.after_sales.order_id,
        Some(ctx.user_id),
        "AFTERSALES.RESEND_SHIPMENT_CREATED",
    )
    .await
    {
        warn!(
            tenant_id = %tenant_id,
            after_sales_id = %after_sales_id,
            order_id = %detail.after_sales.order_id,
            error = %err,
            "failed to refresh profit snapshot after after-sales resend shipment"
        );
    }

    let response = OmsAllocationResponse {
        status: status.to_string(),
        shipments,
        shortages,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_after_sales_import_job(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let mut file_name = None;
    let mut file_bytes = None;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let Some(name) = field.file_name().map(|value| value.to_string()) else {
                    continue;
                };
                let bytes = match field.bytes().await {
                    Ok(v) => v,
                    Err(err) => return (StatusCode::BAD_REQUEST, format!("{err}")).into_response(),
                };
                file_name = Some(name);
                file_bytes = Some(bytes);
                break;
            }
            Ok(None) => break,
            Err(err) => {
                return (StatusCode::BAD_REQUEST, format!("{err}")).into_response();
            }
        }
    }

    let Some(bytes) = file_bytes else {
        return bad_request("file required").into_response();
    };
    let extension = file_name
        .as_deref()
        .and_then(|name| {
            std::path::Path::new(name)
                .extension()
                .and_then(|ext| ext.to_str())
        })
        .unwrap_or("")
        .to_lowercase();
    let file_type = if extension == "csv" {
        "CSV"
    } else if extension == "xls" || extension == "xlsx" {
        "EXCEL"
    } else {
        return bad_request("unsupported file type").into_response();
    };
    let file_id = uuid::Uuid::new_v4();
    let file_path = after_sales_import_file_path(tenant_id, file_id, file_type);
    if let Some(parent) = file_path.parent() {
        if let Err(err) = tokio::fs::create_dir_all(parent).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
    }
    if let Err(err) = tokio::fs::write(&file_path, bytes).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let repo = AfterSalesImportJobsRepo::new(state.db.pool());
    let job = match repo
        .create_job(AfterSalesImportJobCreate {
            tenant_id,
            file_name,
            file_path: file_path.to_string_lossy().to_string(),
            file_type: file_type.to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(after_sales_import_job_view(job))).into_response()
}

pub(crate) async fn list_after_sales_import_jobs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<AfterSalesImportJobListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = AfterSalesImportJobsRepo::new(state.db.pool());
    let result = match repo
        .list_jobs(
            tenant_id,
            AfterSalesImportJobListFilters {
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

    let items = result
        .items
        .into_iter()
        .map(after_sales_import_job_view)
        .collect::<Vec<_>>();
    let response = AfterSalesImportJobListResponse {
        items,
        total: result.total,
        limit,
        offset,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_after_sales_import_job(
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
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };
    let repo = AfterSalesImportJobsRepo::new(state.db.pool());
    let job = match repo.get_job(tenant_id, job_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("import job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(after_sales_import_job_view(job))).into_response()
}

pub(crate) async fn list_refund_docs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<RefundDocListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let status = match query.status.as_deref() {
        Some(raw) => match RefundStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad status").into_response(),
        },
        None => None,
    };
    let after_sales_id = match query.after_sales_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad after_sales_id").into_response(),
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
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = AfterSalesRepo::new(state.db.pool());
    let result = match repo
        .list_refund_docs(
            tenant_id,
            RefundDocListFilters {
                status,
                after_sales_id,
                created_from,
                created_to,
                limit,
                offset,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result.items.iter().map(refund_doc_view).collect();
    let response = RefundDocListResponse {
        items,
        total: result.total,
        limit,
        offset,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_after_sales_intercepts(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<AfterSalesInterceptListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let status = match query.status.as_deref() {
        Some(raw) => match normalize_after_sales_intercept_status(raw) {
            Some(value) => Some(value),
            None => return bad_request("bad status").into_response(),
        },
        None => None,
    };
    let order_id = match query.order_id.as_deref() {
        Some(raw) => match OrderId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad order_id").into_response(),
        },
        None => None,
    };
    let tracking_no = query
        .tracking_no
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let requested_from = match query.requested_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad requested_from").into_response(),
        },
        None => None,
    };
    let requested_to = match query.requested_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad requested_to").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = erp_repo::after_sales_intercepts::AfterSalesInterceptsRepo::new(state.db.pool());
    let result = match repo
        .list_intercepts(
            tenant_id,
            erp_repo::after_sales_intercepts::AfterSalesInterceptListFilters {
                status,
                order_id,
                tracking_no,
                requested_from,
                requested_to,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result
        .items
        .iter()
        .map(after_sales_intercept_view)
        .collect();
    let response = AfterSalesInterceptListResponse {
        items,
        total: result.total,
        limit,
        offset,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_after_sales_intercept(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesInterceptRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let order_id = match OrderId::from_str(&req.order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    let after_sales_id = match req.after_sales_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad after_sales_id").into_response(),
        },
        None => None,
    };
    let status = match req.status.as_deref() {
        Some(raw) => match normalize_after_sales_intercept_status(raw) {
            Some(value) => value,
            None => return bad_request("bad status").into_response(),
        },
        None => "PROCESSING".to_string(),
    };
    let requested_at = match req.requested_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad requested_at").into_response(),
        },
        None => None,
    };

    let repo = erp_repo::after_sales_intercepts::AfterSalesInterceptsRepo::new(state.db.pool());
    let intercept = match repo
        .create_intercept(
            erp_repo::after_sales_intercepts::AfterSalesInterceptCreate {
                tenant_id,
                order_id,
                after_sales_id,
                carrier_name: req
                    .carrier_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                tracking_no: req
                    .tracking_no
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                status,
                reason: req
                    .reason
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                requested_at,
                completed_at: None,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(after_sales_intercept_view(&intercept))).into_response()
}

pub(crate) async fn retry_after_sales_intercept(
    State(state): State<AppState>,
    Path((tenant_id, intercept_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }
    let intercept_id = match uuid::Uuid::parse_str(&intercept_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad intercept_id").into_response(),
    };

    let repo = erp_repo::after_sales_intercepts::AfterSalesInterceptsRepo::new(state.db.pool());
    let updated = match repo
        .update_intercept(
            tenant_id,
            intercept_id,
            erp_repo::after_sales_intercepts::AfterSalesInterceptUpdate {
                status: Some("PROCESSING".to_string()),
                reason: None,
                carrier_name: None,
                tracking_no: None,
                completed_at: None,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("intercept not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(after_sales_intercept_view(&updated))).into_response()
}

pub(crate) async fn escalate_after_sales_intercept(
    State(state): State<AppState>,
    Path((tenant_id, intercept_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }
    let intercept_id = match uuid::Uuid::parse_str(&intercept_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad intercept_id").into_response(),
    };

    let repo = erp_repo::after_sales_intercepts::AfterSalesInterceptsRepo::new(state.db.pool());
    let updated = match repo
        .update_intercept(
            tenant_id,
            intercept_id,
            erp_repo::after_sales_intercepts::AfterSalesInterceptUpdate {
                status: Some("ESCALATED".to_string()),
                reason: None,
                carrier_name: None,
                tracking_no: None,
                completed_at: Some(Utc::now()),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("intercept not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(after_sales_intercept_view(&updated))).into_response()
}

pub(crate) async fn list_after_sales_inspections(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<AfterSalesInspectionListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let after_sales_id = match query.after_sales_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad after_sales_id").into_response(),
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
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let result_code = match query.result_code.as_deref() {
        Some(raw) => match normalize_after_sales_inspection_result(raw) {
            Some(value) => Some(value),
            None => return bad_request("bad result_code").into_response(),
        },
        None => None,
    };
    let inspected_from = match query.inspected_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad inspected_from").into_response(),
        },
        None => None,
    };
    let inspected_to = match query.inspected_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad inspected_to").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = erp_repo::after_sales_inspections::AfterSalesInspectionsRepo::new(state.db.pool());
    let result = match repo
        .list_inspections(
            tenant_id,
            erp_repo::after_sales_inspections::AfterSalesInspectionListFilters {
                after_sales_id,
                sku_id,
                warehouse_id,
                result_code,
                inspected_from,
                inspected_to,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result
        .items
        .iter()
        .map(after_sales_inspection_view)
        .collect();
    let response = AfterSalesInspectionListResponse {
        items,
        total: result.total,
        limit,
        offset,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_after_sales_inspection(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesInspectionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let after_sales_id = match uuid::Uuid::parse_str(&req.after_sales_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    if req.qty <= 0 {
        return bad_request("qty must be positive").into_response();
    }
    let result_code = match req.result_code.as_deref() {
        Some(raw) => match normalize_after_sales_inspection_result(raw) {
            Some(value) => value,
            None => return bad_request("bad result_code").into_response(),
        },
        None => "PENDING".to_string(),
    };
    let inspected_at = match req.inspected_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad inspected_at").into_response(),
        },
        None => None,
    };

    let repo = erp_repo::after_sales_inspections::AfterSalesInspectionsRepo::new(state.db.pool());
    let inspection = match repo
        .create_inspection(
            erp_repo::after_sales_inspections::AfterSalesInspectionCreate {
                tenant_id,
                after_sales_id,
                sku_id,
                warehouse_id,
                qty: req.qty,
                result_code,
                inspector_name: req
                    .inspector_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                inspected_at,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(after_sales_inspection_view(&inspection)),
    )
        .into_response()
}

pub(crate) async fn update_after_sales_inspection(
    State(state): State<AppState>,
    Path((tenant_id, inspection_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesInspectionUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let inspection_id = match uuid::Uuid::parse_str(&inspection_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad inspection_id").into_response(),
    };
    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    if let Some(qty) = req.qty {
        if qty <= 0 {
            return bad_request("qty must be positive").into_response();
        }
    }
    let result_code = match req.result_code.as_deref() {
        Some(raw) => match normalize_after_sales_inspection_result(raw) {
            Some(value) => Some(value),
            None => return bad_request("bad result_code").into_response(),
        },
        None => None,
    };
    let inspected_at = match req.inspected_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad inspected_at").into_response(),
        },
        None => None,
    };

    let repo = erp_repo::after_sales_inspections::AfterSalesInspectionsRepo::new(state.db.pool());
    let inspection = match repo
        .update_inspection(
            tenant_id,
            inspection_id,
            erp_repo::after_sales_inspections::AfterSalesInspectionUpdate {
                warehouse_id,
                qty: req.qty,
                result_code,
                inspector_name: req
                    .inspector_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                inspected_at,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("inspection not found").into_response(),
        Err(e) => {
            if e.to_string().contains("no fields") {
                return bad_request("no fields to update").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    };

    (
        StatusCode::OK,
        Json(after_sales_inspection_view(&inspection)),
    )
        .into_response()
}

pub(crate) async fn list_after_sales_sla_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<AfterSalesSlaRuleListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }

    let after_sales_type = match query.after_sales_type.as_deref() {
        Some(raw) => match AfterSalesType::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad after_sales_type").into_response(),
        },
        None => None,
    };
    let status = match query.status.as_deref() {
        Some(raw) => match normalize_after_sales_sla_status(raw) {
            Some(value) => Some(value),
            None => return bad_request("bad status").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = erp_repo::after_sales_sla_rules::AfterSalesSlaRulesRepo::new(state.db.pool());
    let result = match repo
        .list_rules(
            tenant_id,
            erp_repo::after_sales_sla_rules::AfterSalesSlaRuleListFilters {
                after_sales_type,
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

    let items = result.items.iter().map(after_sales_sla_rule_view).collect();
    let response = AfterSalesSlaRuleListResponse {
        items,
        total: result.total,
        limit,
        offset,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_after_sales_sla_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesSlaRuleRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let after_sales_type = match AfterSalesType::from_str(&req.after_sales_type) {
        Ok(v) => v,
        Err(_) => return bad_request("bad after_sales_type").into_response(),
    };
    if req.response_hours < 0 || req.resolution_hours < 0 {
        return bad_request("hours must be non-negative").into_response();
    }
    let status = match req.status.as_deref() {
        Some(raw) => match normalize_after_sales_sla_status(raw) {
            Some(value) => value,
            None => return bad_request("bad status").into_response(),
        },
        None => "ACTIVE".to_string(),
    };

    let repo = erp_repo::after_sales_sla_rules::AfterSalesSlaRulesRepo::new(state.db.pool());
    let rule = match repo
        .create_rule(erp_repo::after_sales_sla_rules::AfterSalesSlaRuleCreate {
            tenant_id,
            after_sales_type,
            response_hours: req.response_hours,
            resolution_hours: req.resolution_hours,
            escalation_role: req
                .escalation_role
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string()),
            status,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(after_sales_sla_rule_view(&rule))).into_response()
}

pub(crate) async fn update_after_sales_sla_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AfterSalesSlaRuleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Some(response_hours) = req.response_hours {
        if response_hours < 0 {
            return bad_request("response_hours must be non-negative").into_response();
        }
    }
    if let Some(resolution_hours) = req.resolution_hours {
        if resolution_hours < 0 {
            return bad_request("resolution_hours must be non-negative").into_response();
        }
    }
    let after_sales_type = match req.after_sales_type.as_deref() {
        Some(raw) => match AfterSalesType::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad after_sales_type").into_response(),
        },
        None => None,
    };
    let status = match req.status.as_deref() {
        Some(raw) => match normalize_after_sales_sla_status(raw) {
            Some(value) => Some(value),
            None => return bad_request("bad status").into_response(),
        },
        None => None,
    };

    let repo = erp_repo::after_sales_sla_rules::AfterSalesSlaRulesRepo::new(state.db.pool());
    let rule = match repo
        .update_rule(
            tenant_id,
            rule_id,
            erp_repo::after_sales_sla_rules::AfterSalesSlaRuleUpdate {
                after_sales_type,
                response_hours: req.response_hours,
                resolution_hours: req.resolution_hours,
                escalation_role: req
                    .escalation_role
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                status,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sla rule not found").into_response(),
        Err(e) => {
            if e.to_string().contains("no fields") {
                return bad_request("no fields to update").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    };

    (StatusCode::OK, Json(after_sales_sla_rule_view(&rule))).into_response()
}

pub(crate) async fn delete_after_sales_sla_rule(
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
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };

    let repo = erp_repo::after_sales_sla_rules::AfterSalesSlaRulesRepo::new(state.db.pool());
    let deleted = match repo.delete_rule(tenant_id, rule_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sla rule not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(after_sales_sla_rule_view(&deleted))).into_response()
}

pub(crate) fn after_sales_view(sale: &erp_repo::after_sales::AfterSales) -> AfterSalesView {
    after_sales_view_with_sla(sale, AfterSalesSlaView::default())
}

#[derive(Debug, Clone, Default)]
struct AfterSalesSlaView {
    sla_deadline_at: Option<DateTime<Utc>>,
    sla_status: Option<String>,
    sla_remaining_minutes: Option<i64>,
    escalation_role: Option<String>,
}

#[derive(Debug, Clone)]
struct AfterSalesSlaProfile {
    resolution_hours: i32,
    escalation_role: Option<String>,
}

async fn load_after_sales_sla_profiles(
    pool: &PgPool,
    tenant_id: TenantId,
) -> HashMap<String, AfterSalesSlaProfile> {
    let repo = erp_repo::after_sales_sla_rules::AfterSalesSlaRulesRepo::new(pool);
    let result = repo
        .list_rules(
            tenant_id,
            erp_repo::after_sales_sla_rules::AfterSalesSlaRuleListFilters {
                after_sales_type: None,
                status: Some("ACTIVE".to_string()),
                offset: 0,
                limit: 200,
            },
        )
        .await;
    match result {
        Ok(v) => v
            .items
            .into_iter()
            .map(|rule| {
                (
                    rule.after_sales_type.to_string(),
                    AfterSalesSlaProfile {
                        resolution_hours: rule.resolution_hours.max(1),
                        escalation_role: rule.escalation_role,
                    },
                )
            })
            .collect(),
        Err(err) => {
            warn!(error = %err, tenant_id = %tenant_id, "load after-sales sla rules failed");
            HashMap::new()
        }
    }
}

pub(crate) async fn count_after_sales_sla_overdue_snapshot(
    pool: &PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<i64> {
    let overdue_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from after_sales s
        left join lateral (
            select r.resolution_hours
            from after_sales_sla_rules r
            where r.tenant_id = s.tenant_id
              and upper(r.status) = 'ACTIVE'
              and upper(r.after_sales_type) = upper(s.type)
            order by r.updated_at desc, r.rule_id desc
            limit 1
        ) rule on true
        where s.tenant_id = $1
          and upper(s.status) not in ('REFUNDED', 'CLOSED', 'REJECTED')
          and s.created_at + make_interval(hours => greatest(coalesce(rule.resolution_hours, 24), 1))
              < now()
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(pool)
    .await?;
    Ok(overdue_count)
}

fn derive_after_sales_sla_view(
    sale: &erp_repo::after_sales::AfterSales,
    profiles: &HashMap<String, AfterSalesSlaProfile>,
    now: DateTime<Utc>,
) -> AfterSalesSlaView {
    if matches!(
        sale.status,
        AfterSalesStatus::Refunded | AfterSalesStatus::Closed | AfterSalesStatus::Rejected
    ) {
        return AfterSalesSlaView {
            sla_deadline_at: None,
            sla_status: Some("CLOSED".to_string()),
            sla_remaining_minutes: Some(0),
            escalation_role: None,
        };
    }

    let profile = profiles.get(&sale.after_sales_type.to_string());
    let resolution_hours = profile
        .map(|rule| rule.resolution_hours)
        .unwrap_or(24)
        .max(1);
    let deadline = sale.created_at + Duration::hours(i64::from(resolution_hours));
    let remaining_minutes = (deadline - now).num_minutes();
    let sla_status = if remaining_minutes < 0 {
        "OVERDUE"
    } else if remaining_minutes <= 60 {
        "DUE_SOON"
    } else {
        "ON_TRACK"
    };

    AfterSalesSlaView {
        sla_deadline_at: Some(deadline),
        sla_status: Some(sla_status.to_string()),
        sla_remaining_minutes: Some(remaining_minutes),
        escalation_role: profile.and_then(|rule| rule.escalation_role.clone()),
    }
}

fn after_sales_view_with_sla(
    sale: &erp_repo::after_sales::AfterSales,
    sla: AfterSalesSlaView,
) -> AfterSalesView {
    AfterSalesView {
        id: sale.id,
        tenant_id: uuid::Uuid::from(sale.tenant_id),
        order_id: uuid::Uuid::from(sale.order_id),
        after_sales_type: sale.after_sales_type.to_string(),
        status: sale.status.to_string(),
        after_sale_status: sale.after_sale_status.clone(),
        compensation_status: sale.compensation_status.clone(),
        evidence_status: sale.evidence_status.clone(),
        reason_code: sale.reason_code.clone(),
        amount_cent: sale.amount_cent,
        sla_deadline_at: sla.sla_deadline_at,
        sla_status: sla.sla_status,
        sla_remaining_minutes: sla.sla_remaining_minutes,
        escalation_role: sla.escalation_role,
        created_at: sale.created_at,
        updated_at: sale.updated_at,
    }
}

pub(crate) fn after_sales_item_view(
    item: &erp_repo::after_sales::AfterSalesItem,
) -> AfterSalesItemView {
    AfterSalesItemView {
        sku_id: item.sku_id,
        qty: item.qty,
        refund_cent: item.refund_cent,
    }
}

pub(crate) fn return_receipt_view(
    receipt: &erp_repo::after_sales::ReturnReceipt,
) -> ReturnReceiptView {
    ReturnReceiptView {
        id: receipt.id,
        after_sales_id: receipt.after_sales_id,
        warehouse_id: receipt.warehouse_id,
        status: receipt.status.to_string(),
        return_status: receipt.return_status.clone(),
        received_at: receipt.received_at,
        putaway_at: receipt.putaway_at,
        created_at: receipt.created_at,
        updated_at: receipt.updated_at,
    }
}

pub(crate) fn refund_doc_view(doc: &erp_repo::after_sales::RefundDoc) -> RefundDocView {
    RefundDocView {
        id: doc.id,
        after_sales_id: doc.after_sales_id,
        amount_cent: doc.amount_cent,
        status: doc.status.to_string(),
        refund_status: doc.refund_status.clone(),
        channel: doc.channel.to_string(),
        created_at: doc.created_at,
        updated_at: doc.updated_at,
    }
}

pub(crate) fn after_sales_detail_view(
    detail: erp_repo::after_sales::AfterSalesDetail,
    audit_logs: Vec<erp_repo::audit_logs::AuditLog>,
) -> AfterSalesDetailView {
    AfterSalesDetailView {
        after_sales: after_sales_view(&detail.after_sales),
        items: detail.items.iter().map(after_sales_item_view).collect(),
        return_receipt: detail.return_receipt.as_ref().map(return_receipt_view),
        refund_doc: detail.refund_doc.as_ref().map(refund_doc_view),
        shipments: detail
            .shipments
            .iter()
            .map(|shipment| AfterSalesShipmentView {
                shipment_id: shipment.shipment_id,
                shipment_type: shipment.shipment_type.clone(),
                origin_shipment_id: shipment.origin_shipment_id,
                created_at: shipment.created_at,
            })
            .collect(),
        raw_json: detail.after_sales.raw_json,
        audit_logs: audit_logs
            .into_iter()
            .map(|log| AfterSalesAuditLogView {
                audit_id: log.audit_id,
                action: log.action,
                user_id: log.user_id.map(uuid::Uuid::from),
                before_json: log.before_json,
                after_json: log.after_json,
                created_at: log.created_at,
            })
            .collect(),
    }
}

pub(crate) fn after_sales_import_job_view(
    job: erp_repo::after_sales_import_jobs::AfterSalesImportJob,
) -> AfterSalesImportJobView {
    AfterSalesImportJobView {
        job_id: job.job_id,
        file_name: job.file_name,
        file_type: job.file_type,
        status: job.status,
        progress: job.progress,
        total_rows: job.total_rows,
        success_rows: job.success_rows,
        failed_rows: job.failed_rows,
        attempts: job.attempts,
        last_error: job.last_error,
        created_at: job.created_at,
        updated_at: job.updated_at,
    }
}

pub(crate) fn after_sales_intercept_view(
    intercept: &erp_repo::after_sales_intercepts::AfterSalesIntercept,
) -> AfterSalesInterceptView {
    AfterSalesInterceptView {
        intercept_id: intercept.intercept_id,
        order_id: uuid::Uuid::from(intercept.order_id),
        after_sales_id: intercept.after_sales_id,
        carrier_name: intercept.carrier_name.clone(),
        tracking_no: intercept.tracking_no.clone(),
        status: intercept.status.clone(),
        reason: intercept.reason.clone(),
        requested_at: intercept.requested_at,
        completed_at: intercept.completed_at,
        created_at: intercept.created_at,
        updated_at: intercept.updated_at,
    }
}

pub(crate) fn after_sales_inspection_view(
    inspection: &erp_repo::after_sales_inspections::AfterSalesInspection,
) -> AfterSalesInspectionView {
    AfterSalesInspectionView {
        inspection_id: inspection.inspection_id,
        after_sales_id: inspection.after_sales_id,
        sku_id: inspection.sku_id,
        warehouse_id: inspection.warehouse_id,
        qty: inspection.qty,
        result_code: inspection.result_code.clone(),
        inspector_name: inspection.inspector_name.clone(),
        inspected_at: inspection.inspected_at,
        created_at: inspection.created_at,
        updated_at: inspection.updated_at,
    }
}

pub(crate) fn after_sales_sla_rule_view(
    rule: &erp_repo::after_sales_sla_rules::AfterSalesSlaRule,
) -> AfterSalesSlaRuleView {
    AfterSalesSlaRuleView {
        rule_id: rule.rule_id,
        after_sales_type: rule.after_sales_type.to_string(),
        response_hours: rule.response_hours,
        resolution_hours: rule.resolution_hours,
        escalation_role: rule.escalation_role.clone(),
        status: rule.status.clone(),
        created_at: rule.created_at,
        updated_at: rule.updated_at,
    }
}

pub(crate) fn normalize_after_sales_intercept_status(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    match value.as_str() {
        "PROCESSING" | "SUCCESS" | "FAILED" | "ESCALATED" => Some(value),
        _ => None,
    }
}

pub(crate) fn normalize_after_sales_inspection_result(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    match value.as_str() {
        "PENDING" | "RESALE" | "REPAIR" | "SCRAP" => Some(value),
        _ => None,
    }
}

pub(crate) fn normalize_after_sales_sla_status(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    match value.as_str() {
        "ACTIVE" | "DISABLED" => Some(value),
        _ => None,
    }
}

pub(crate) async fn load_after_sales_audit_logs(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    after_sales_id: uuid::Uuid,
    refund_doc_id: Option<uuid::Uuid>,
    limit: i64,
) -> anyhow::Result<Vec<erp_repo::audit_logs::AuditLog>> {
    let repo = AuditLogsRepo::new(pool);
    let mut logs = repo
        .list_logs(
            tenant_id,
            AuditLogListFilters {
                action: None,
                entity_type: Some("after_sales".to_string()),
                entity_id: Some(after_sales_id.to_string()),
                user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit,
            },
        )
        .await?
        .items;

    if let Some(refund_id) = refund_doc_id {
        let refund_logs = repo
            .list_logs(
                tenant_id,
                AuditLogListFilters {
                    action: None,
                    entity_type: Some("refund_doc".to_string()),
                    entity_id: Some(refund_id.to_string()),
                    user_id: None,
                    created_from: None,
                    created_to: None,
                    cursor: None,
                    limit,
                },
            )
            .await?
            .items;
        logs.extend(refund_logs);
    }

    logs.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.audit_id.cmp(&a.audit_id))
    });
    if logs.len() > limit as usize {
        logs.truncate(limit as usize);
    }
    Ok(logs)
}

pub(crate) fn after_sales_repo_error(err: anyhow::Error) -> Response {
    let message = err.to_string();
    if message.contains("return receipt not found") {
        return not_found("return receipt not found").into_response();
    }
    if message.contains("not found") {
        return not_found("after_sales not found").into_response();
    }
    if message.contains("status not allowed") || message.contains("status not receivable") {
        return conflict(&message).into_response();
    }
    if message.contains("status not cancellable") {
        return conflict(&message).into_response();
    }
    if message.contains("items empty") {
        return bad_request("after_sales items empty").into_response();
    }
    (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
}

pub(crate) fn normalize_directory_status(status: Option<&str>) -> String {
    match status.map(|value| value.trim().to_uppercase()) {
        Some(value) if value == "DISABLED" => "DISABLED".to_string(),
        _ => "ACTIVE".to_string(),
    }
}

pub(crate) fn normalize_quality_result_code(raw: &str) -> Option<String> {
    match raw.trim().to_uppercase().as_str() {
        "RESALE" => Some("RESALE".to_string()),
        "SCRAP" => Some("SCRAP".to_string()),
        "REPAIR" => Some("REPAIR".to_string()),
        _ => None,
    }
}
