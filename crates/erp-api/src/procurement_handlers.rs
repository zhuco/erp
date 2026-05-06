use super::*;

pub(crate) async fn list_purchase_orders(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PurchaseOrderListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_orders.read") {
        return resp.into_response();
    }

    let supplier_id = match query.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let status = match query.status.as_deref() {
        Some(raw) => match normalize_purchase_order_status(raw) {
            Some(v) => Some(v),
            None => return bad_request("bad status").into_response(),
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

    let repo = PurchaseOrdersRepo::new(state.db.pool());
    let result = match repo
        .list_orders(
            tenant_id,
            PurchaseOrderListFilters {
                supplier_id,
                warehouse_id,
                status,
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

    let response = PurchaseOrderListResponse {
        items: result
            .items
            .into_iter()
            .map(purchase_order_summary_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_purchase_order(
    State(state): State<AppState>,
    Path((tenant_id, po_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let po_id = match uuid::Uuid::parse_str(&po_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad po_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_orders.read") {
        return resp.into_response();
    }

    let repo = PurchaseOrdersRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, po_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("purchase order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(purchase_order_detail_response(detail))).into_response()
}

pub(crate) async fn create_purchase_order(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PurchaseOrderCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_orders.write") {
        return resp.into_response();
    }

    let supplier_id = match uuid::Uuid::parse_str(req.supplier_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad supplier_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let expected_at = match req.expected_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad expected_at").into_response(),
        },
        None => None,
    };
    let remark = req.remark.clone().map(|value| value.trim().to_string());

    let mut items = Vec::with_capacity(req.items.len());
    for item in req.items {
        let sku_id = match uuid::Uuid::parse_str(item.sku_id.trim()) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(PurchaseOrderItemInput {
            sku_id,
            qty: item.qty,
            price_cent: item.price_cent,
        });
    }

    let repo = PurchaseOrdersRepo::new(state.db.pool());
    let detail = match repo
        .create_order(PurchaseOrderCreate {
            tenant_id,
            supplier_id,
            warehouse_id,
            expected_at,
            remark,
            items,
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
            action: "PROCUREMENT.PO_CREATE".to_string(),
            entity_type: "purchase_order".to_string(),
            entity_id: Some(detail.order.po_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&detail).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write purchase order audit log");
    }

    (StatusCode::OK, Json(purchase_order_detail_response(detail))).into_response()
}

pub(crate) async fn update_purchase_order(
    State(state): State<AppState>,
    Path((tenant_id, po_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PurchaseOrderUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let po_id = match uuid::Uuid::parse_str(&po_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad po_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_orders.write") {
        return resp.into_response();
    }

    let supplier_id = match req.supplier_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let expected_at = match req.expected_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad expected_at").into_response(),
        },
        None => None,
    };
    let remark = req.remark.clone().map(|value| value.trim().to_string());

    let items = match req.items {
        Some(items) => {
            let mut converted = Vec::with_capacity(items.len());
            for item in items {
                let sku_id = match uuid::Uuid::parse_str(item.sku_id.trim()) {
                    Ok(v) => v,
                    Err(_) => return bad_request("bad sku_id").into_response(),
                };
                converted.push(PurchaseOrderItemInput {
                    sku_id,
                    qty: item.qty,
                    price_cent: item.price_cent,
                });
            }
            Some(converted)
        }
        None => None,
    };

    let repo = PurchaseOrdersRepo::new(state.db.pool());
    let detail = match repo
        .update_order(
            tenant_id,
            po_id,
            PurchaseOrderUpdate {
                supplier_id,
                warehouse_id,
                expected_at,
                remark,
                items,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("purchase order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "PROCUREMENT.PO_UPDATE".to_string(),
            entity_type: "purchase_order".to_string(),
            entity_id: Some(detail.order.po_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&detail).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write purchase order audit log");
    }

    (StatusCode::OK, Json(purchase_order_detail_response(detail))).into_response()
}

pub(crate) async fn delete_purchase_order(
    State(state): State<AppState>,
    Path((tenant_id, po_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let po_id = match uuid::Uuid::parse_str(&po_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad po_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_orders.write") {
        return resp.into_response();
    }

    let repo = PurchaseOrdersRepo::new(state.db.pool());
    let order = match repo.delete_order(tenant_id, po_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("purchase order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "PROCUREMENT.PO_DELETE".to_string(),
            entity_type: "purchase_order".to_string(),
            entity_id: Some(order.po_id.to_string()),
            before_json: serde_json::to_value(&order).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write purchase order audit log");
    }

    (StatusCode::OK, Json(purchase_order_summary_response(order))).into_response()
}

pub(crate) async fn send_purchase_order(
    State(state): State<AppState>,
    Path((tenant_id, po_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let po_id = match uuid::Uuid::parse_str(&po_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad po_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_orders.write") {
        return resp.into_response();
    }

    let repo = PurchaseOrdersRepo::new(state.db.pool());
    let order = match repo.mark_sent(tenant_id, po_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return bad_request("purchase order not sendable").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "PROCUREMENT.PO_SEND".to_string(),
            entity_type: "purchase_order".to_string(),
            entity_id: Some(order.po_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&order).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write purchase order audit log");
    }

    (StatusCode::OK, Json(purchase_order_summary_response(order))).into_response()
}

pub(crate) async fn confirm_purchase_order(
    State(state): State<AppState>,
    Path((tenant_id, po_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let po_id = match uuid::Uuid::parse_str(&po_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad po_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_orders.write") {
        return resp.into_response();
    }

    let repo = PurchaseOrdersRepo::new(state.db.pool());
    let order = match repo.confirm_order(tenant_id, po_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return bad_request("purchase order not confirmable").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "PROCUREMENT.PO_CONFIRM".to_string(),
            entity_type: "purchase_order".to_string(),
            entity_id: Some(order.po_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&order).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write purchase order audit log");
    }

    (StatusCode::OK, Json(purchase_order_summary_response(order))).into_response()
}

pub(crate) async fn receive_purchase_order(
    State(state): State<AppState>,
    Path((tenant_id, po_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PurchaseOrderReceiveRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let po_id = match uuid::Uuid::parse_str(&po_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad po_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_orders.write") {
        return resp.into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let mut items = Vec::with_capacity(req.items.len());
    for item in req.items {
        let sku_id = match uuid::Uuid::parse_str(item.sku_id.trim()) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(PurchaseOrderReceiveItem {
            sku_id,
            qty: item.qty,
        });
    }

    let cost_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::CostManagement).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    let update_cost = req.update_cost.unwrap_or(true) && cost_enabled;

    let repo = PurchaseOrdersRepo::new(state.db.pool());
    let result = match repo
        .receive_order(PurchaseOrderReceiveInput {
            tenant_id,
            po_id,
            warehouse_id,
            items,
            update_cost,
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
            action: "PROCUREMENT.PO_RECEIVE".to_string(),
            entity_type: "purchase_order".to_string(),
            entity_id: Some(result.order.po_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&result).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write purchase order audit log");
    }

    let response = PurchaseOrderReceiveResponse {
        order: purchase_order_summary_response(result.order),
        receipt: purchase_receipt_response(result.receipt),
        receipt_items: result
            .receipt_items
            .into_iter()
            .map(purchase_receipt_item_response)
            .collect(),
        inbound_doc_id: result.inbound_doc_id,
        ap_doc_id: result.ap_doc_id,
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_purchase_receipts(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PurchaseReceiptListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_orders.read") {
        return resp.into_response();
    }

    let supplier_id = match query.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let status = match query.status.as_deref() {
        Some(raw) => match normalize_purchase_receipt_status(raw) {
            Some(v) => Some(v),
            None => return bad_request("bad status").into_response(),
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

    let repo = PurchaseReceiptsRepo::new(state.db.pool());
    let result = match repo
        .list_receipts(
            tenant_id,
            PurchaseReceiptListFilters {
                supplier_id,
                warehouse_id,
                status,
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

    let response = PurchaseReceiptListResponse {
        items: result
            .items
            .into_iter()
            .map(purchase_receipt_summary_response)
            .collect(),
        total: result.total,
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_purchase_receipt_detail(
    State(state): State<AppState>,
    Path((tenant_id, receipt_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let receipt_id = match uuid::Uuid::parse_str(&receipt_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad receipt_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_orders.read") {
        return resp.into_response();
    }

    let repo = PurchaseReceiptsRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, receipt_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("purchase receipt not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(purchase_receipt_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn list_purchase_suggestions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PurchaseSuggestionListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_suggestions.read") {
        return resp.into_response();
    }

    let sku_id = match query.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = PurchaseSuggestionsRepo::new(state.db.pool());
    let _ = repo.refresh(tenant_id).await;
    let result = match repo
        .list_suggestions(
            tenant_id,
            PurchaseSuggestionListFilters {
                sku_id,
                reason: query.reason.clone(),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = PurchaseSuggestionListResponse {
        items: result
            .items
            .into_iter()
            .map(purchase_suggestion_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_purchase_returns(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PurchaseReturnListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_returns.read") {
        return resp.into_response();
    }

    let supplier_id = match query.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let status = match query.status.as_deref() {
        Some(raw) => match normalize_purchase_return_status(raw) {
            Some(v) => Some(v),
            None => return bad_request("bad status").into_response(),
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

    let repo = PurchaseReturnsRepo::new(state.db.pool());
    let result = match repo
        .list_returns(
            tenant_id,
            PurchaseReturnListFilters {
                supplier_id,
                warehouse_id,
                status,
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

    let response = PurchaseReturnListResponse {
        items: result
            .items
            .into_iter()
            .map(purchase_return_summary_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_purchase_return_detail(
    State(state): State<AppState>,
    Path((tenant_id, return_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let return_id = match uuid::Uuid::parse_str(&return_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad return_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_returns.read") {
        return resp.into_response();
    }

    let repo = PurchaseReturnsRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, return_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("purchase return not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(purchase_return_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn create_purchase_return(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PurchaseReturnCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_returns.write") {
        return resp.into_response();
    }

    let po_id = match uuid::Uuid::parse_str(req.po_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad po_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };

    let mut items = Vec::with_capacity(req.items.len());
    for item in req.items {
        let sku_id = match uuid::Uuid::parse_str(item.sku_id.trim()) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(PurchaseReturnItem {
            sku_id,
            qty: item.qty,
        });
    }

    let repo = PurchaseReturnsRepo::new(state.db.pool());
    let detail = match repo
        .create_return(PurchaseReturnCreate {
            tenant_id,
            po_id,
            warehouse_id,
            items,
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
            action: "PROCUREMENT.RETURN_CREATE".to_string(),
            entity_type: "purchase_return".to_string(),
            entity_id: Some(detail.purchase_return.return_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&detail).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write purchase return audit log");
    }

    (
        StatusCode::OK,
        Json(purchase_return_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn confirm_purchase_return(
    State(state): State<AppState>,
    Path((tenant_id, return_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let return_id = match uuid::Uuid::parse_str(&return_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad return_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "purchase_returns.write") {
        return resp.into_response();
    }

    let repo = PurchaseReturnsRepo::new(state.db.pool());
    let result = match repo.confirm_return(tenant_id, return_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "PROCUREMENT.RETURN_CONFIRM".to_string(),
            entity_type: "purchase_return".to_string(),
            entity_id: Some(result.purchase_return.return_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&result).ok(),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "failed to write purchase return audit log");
    }

    let response = PurchaseReturnConfirmResponse {
        purchase_return: purchase_return_summary_response(result.purchase_return),
        outbound_doc_id: result.outbound_doc_id,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_procurement_requests(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProcurementRequestListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_requests.read") {
        return resp.into_response();
    }

    let created_from = match parse_datetime_optional(query.created_from.as_deref(), "created_from")
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let created_to = match parse_datetime_optional(query.created_to.as_deref(), "created_to") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let department = normalize_optional_string(query.department);
    let requester = normalize_optional_string(query.requester);
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = ProcurementRequestsRepo::new(state.db.pool());
    let result = match repo
        .list_requests(
            tenant_id,
            ProcurementRequestListFilters {
                department,
                requester,
                status,
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

    let response = ProcurementRequestListResponse {
        items: result
            .items
            .into_iter()
            .map(procurement_request_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_procurement_request_detail(
    State(state): State<AppState>,
    Path((tenant_id, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let request_id = match uuid::Uuid::parse_str(&request_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad request_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_requests.read") {
        return resp.into_response();
    }

    let repo = ProcurementRequestsRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, request_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement request not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_request_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn create_procurement_request(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementRequestCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_requests.write") {
        return resp.into_response();
    }
    if req.items.is_empty() {
        return bad_request("items required").into_response();
    }
    let expected_at = match parse_datetime_optional(req.expected_at.as_deref(), "expected_at") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let mut items = Vec::with_capacity(req.items.len());
    for item in req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(ProcurementRequestItemInput {
            sku_id,
            qty: item.qty,
            price_cent: item.price_cent,
        });
    }

    let repo = ProcurementRequestsRepo::new(state.db.pool());
    let detail = match repo
        .create_request(ProcurementRequestCreate {
            tenant_id,
            department: normalize_optional_string(req.department),
            requester: normalize_optional_string(req.requester),
            expected_at,
            remark: normalize_optional_string(req.remark),
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_request_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn update_procurement_request(
    State(state): State<AppState>,
    Path((tenant_id, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementRequestUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let request_id = match uuid::Uuid::parse_str(&request_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad request_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_requests.write") {
        return resp.into_response();
    }

    let expected_at = match parse_datetime_optional(req.expected_at.as_deref(), "expected_at") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let items = match req.items {
        Some(items) => {
            if items.is_empty() {
                return bad_request("items required").into_response();
            }
            let mut converted = Vec::with_capacity(items.len());
            for item in items {
                let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
                    Ok(v) => v,
                    Err(_) => return bad_request("bad sku_id").into_response(),
                };
                converted.push(ProcurementRequestItemInput {
                    sku_id,
                    qty: item.qty,
                    price_cent: item.price_cent,
                });
            }
            Some(converted)
        }
        None => None,
    };

    let repo = ProcurementRequestsRepo::new(state.db.pool());
    let detail = match repo
        .update_request(
            tenant_id,
            request_id,
            ProcurementRequestUpdate {
                department: normalize_optional_string(req.department),
                requester: normalize_optional_string(req.requester),
                status,
                expected_at,
                remark: normalize_optional_string(req.remark),
                items,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement request not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_request_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn delete_procurement_request(
    State(state): State<AppState>,
    Path((tenant_id, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let request_id = match uuid::Uuid::parse_str(&request_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad request_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_requests.write") {
        return resp.into_response();
    }

    let repo = ProcurementRequestsRepo::new(state.db.pool());
    let request = match repo.delete_request(tenant_id, request_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement request not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(procurement_request_response(request))).into_response()
}

pub(crate) async fn list_procurement_plans(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProcurementPlanListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_plans.read") {
        return resp.into_response();
    }

    let supplier_id = match query.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let created_from = match parse_datetime_optional(query.created_from.as_deref(), "created_from")
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let created_to = match parse_datetime_optional(query.created_to.as_deref(), "created_to") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = ProcurementPlansRepo::new(state.db.pool());
    let result = match repo
        .list_plans(
            tenant_id,
            ProcurementPlanListFilters {
                supplier_id,
                status,
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

    let response = ProcurementPlanListResponse {
        items: result
            .items
            .into_iter()
            .map(procurement_plan_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_procurement_plan_detail(
    State(state): State<AppState>,
    Path((tenant_id, plan_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let plan_id = match uuid::Uuid::parse_str(&plan_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad plan_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_plans.read") {
        return resp.into_response();
    }

    let repo = ProcurementPlansRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, plan_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement plan not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_plan_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn generate_procurement_plan(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementPlanGenerateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_plans.write") {
        return resp.into_response();
    }

    let request_id = match req.request_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad request_id").into_response(),
        },
        None => None,
    };
    let supplier_id = match req.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let sku_id = match req.sku_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let planned_at = match parse_datetime_optional(req.planned_at.as_deref(), "planned_at") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let expected_at = match parse_datetime_optional(req.expected_at.as_deref(), "expected_at") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let max_items = req.max_items.unwrap_or(200).clamp(1, 500);
    let min_suggested_qty = req.min_suggested_qty.unwrap_or(1).max(1);
    let default_price_cent = req.default_price_cent.unwrap_or(0).max(0);

    let suggestions_repo = PurchaseSuggestionsRepo::new(state.db.pool());
    let suggestions = match suggestions_repo
        .list_suggestions(
            tenant_id,
            PurchaseSuggestionListFilters {
                sku_id,
                reason: normalize_optional_string(req.reason),
                offset: 0,
                limit: max_items,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if suggestions.items.is_empty() {
        return bad_request("purchase suggestions empty").into_response();
    }

    let mut qty_by_sku: std::collections::BTreeMap<uuid::Uuid, i64> =
        std::collections::BTreeMap::new();
    for suggestion in suggestions.items {
        if suggestion.suggested_qty >= min_suggested_qty {
            *qty_by_sku.entry(suggestion.sku_id).or_insert(0) += suggestion.suggested_qty;
        }
    }
    if qty_by_sku.is_empty() {
        return bad_request("no suggestions can generate plan").into_response();
    }

    let items: Vec<ProcurementPlanItemInput> = qty_by_sku
        .into_iter()
        .map(|(sku_id, qty)| ProcurementPlanItemInput {
            sku_id,
            qty,
            price_cent: default_price_cent,
        })
        .collect();

    let remark = normalize_optional_string(req.remark)
        .or(Some("AUTO_GENERATED_FROM_PURCHASE_SUGGESTIONS".to_string()));
    let repo = ProcurementPlansRepo::new(state.db.pool());
    let detail = match repo
        .create_plan(ProcurementPlanCreate {
            tenant_id,
            request_id,
            supplier_id,
            planned_at,
            expected_at,
            remark,
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_plan_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn create_procurement_plan(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementPlanCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_plans.write") {
        return resp.into_response();
    }
    if req.items.is_empty() {
        return bad_request("items required").into_response();
    }

    let request_id = match req.request_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad request_id").into_response(),
        },
        None => None,
    };
    let supplier_id = match req.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let planned_at = match parse_datetime_optional(req.planned_at.as_deref(), "planned_at") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let expected_at = match parse_datetime_optional(req.expected_at.as_deref(), "expected_at") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let mut items = Vec::with_capacity(req.items.len());
    for item in req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(ProcurementPlanItemInput {
            sku_id,
            qty: item.qty,
            price_cent: item.price_cent,
        });
    }

    let repo = ProcurementPlansRepo::new(state.db.pool());
    let detail = match repo
        .create_plan(ProcurementPlanCreate {
            tenant_id,
            request_id,
            supplier_id,
            planned_at,
            expected_at,
            remark: normalize_optional_string(req.remark),
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_plan_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn update_procurement_plan(
    State(state): State<AppState>,
    Path((tenant_id, plan_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementPlanUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let plan_id = match uuid::Uuid::parse_str(&plan_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad plan_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_plans.write") {
        return resp.into_response();
    }

    let supplier_id = match req.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let planned_at = match parse_datetime_optional(req.planned_at.as_deref(), "planned_at") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let expected_at = match parse_datetime_optional(req.expected_at.as_deref(), "expected_at") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let items = match req.items {
        Some(items) => {
            if items.is_empty() {
                return bad_request("items required").into_response();
            }
            let mut converted = Vec::with_capacity(items.len());
            for item in items {
                let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
                    Ok(v) => v,
                    Err(_) => return bad_request("bad sku_id").into_response(),
                };
                converted.push(ProcurementPlanItemInput {
                    sku_id,
                    qty: item.qty,
                    price_cent: item.price_cent,
                });
            }
            Some(converted)
        }
        None => None,
    };

    let repo = ProcurementPlansRepo::new(state.db.pool());
    let detail = match repo
        .update_plan(
            tenant_id,
            plan_id,
            ProcurementPlanUpdate {
                supplier_id,
                status,
                planned_at,
                expected_at,
                remark: normalize_optional_string(req.remark),
                items,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement plan not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_plan_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn delete_procurement_plan(
    State(state): State<AppState>,
    Path((tenant_id, plan_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let plan_id = match uuid::Uuid::parse_str(&plan_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad plan_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_plans.write") {
        return resp.into_response();
    }

    let repo = ProcurementPlansRepo::new(state.db.pool());
    let plan = match repo.delete_plan(tenant_id, plan_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement plan not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(procurement_plan_response(plan))).into_response()
}

pub(crate) async fn list_procurement_sourcing(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProcurementSourcingListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_sourcing.read") {
        return resp.into_response();
    }

    let supplier_id = match query.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let created_from = match parse_datetime_optional(query.created_from.as_deref(), "created_from")
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let created_to = match parse_datetime_optional(query.created_to.as_deref(), "created_to") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let keyword = normalize_optional_string(query.keyword);
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = ProcurementSourcingRepo::new(state.db.pool());
    let result = match repo
        .list_sourcing(
            tenant_id,
            ProcurementSourcingListFilters {
                keyword,
                supplier_id,
                status,
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

    let response = ProcurementSourcingListResponse {
        items: result
            .items
            .into_iter()
            .map(procurement_sourcing_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_procurement_sourcing(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementSourcingCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_sourcing.write") {
        return resp.into_response();
    }
    let title = req.title.trim();
    if title.is_empty() {
        return bad_request("title is required").into_response();
    }

    let supplier_id = match req.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let repo = ProcurementSourcingRepo::new(state.db.pool());
    let sourcing = match repo
        .create_sourcing(ProcurementSourcingCreate {
            tenant_id,
            title: title.to_string(),
            category: normalize_optional_string(req.category),
            supplier_id,
            status,
            target_price_cent: req.target_price_cent,
            remark: normalize_optional_string(req.remark),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_sourcing_response(sourcing)),
    )
        .into_response()
}

pub(crate) async fn update_procurement_sourcing(
    State(state): State<AppState>,
    Path((tenant_id, sourcing_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementSourcingUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let sourcing_id = match uuid::Uuid::parse_str(&sourcing_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sourcing_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_sourcing.write") {
        return resp.into_response();
    }

    let supplier_id = match req.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let title = req
        .title
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let repo = ProcurementSourcingRepo::new(state.db.pool());
    let sourcing = match repo
        .update_sourcing(
            tenant_id,
            sourcing_id,
            ProcurementSourcingUpdate {
                title,
                category: normalize_optional_string(req.category),
                supplier_id,
                status,
                target_price_cent: req.target_price_cent,
                remark: normalize_optional_string(req.remark),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement sourcing not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_sourcing_response(sourcing)),
    )
        .into_response()
}

pub(crate) async fn delete_procurement_sourcing(
    State(state): State<AppState>,
    Path((tenant_id, sourcing_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let sourcing_id = match uuid::Uuid::parse_str(&sourcing_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sourcing_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_sourcing.write") {
        return resp.into_response();
    }

    let repo = ProcurementSourcingRepo::new(state.db.pool());
    let sourcing = match repo.delete_sourcing(tenant_id, sourcing_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement sourcing not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_sourcing_response(sourcing)),
    )
        .into_response()
}

pub(crate) async fn list_procurement_inspections(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProcurementInspectionListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_inspections.read") {
        return resp.into_response();
    }

    let supplier_id = match query.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let created_from = match parse_datetime_optional(query.created_from.as_deref(), "created_from")
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let created_to = match parse_datetime_optional(query.created_to.as_deref(), "created_to") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = ProcurementInspectionsRepo::new(state.db.pool());
    let result = match repo
        .list_inspections(
            tenant_id,
            ProcurementInspectionListFilters {
                supplier_id,
                warehouse_id,
                status,
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

    let response = ProcurementInspectionListResponse {
        items: result
            .items
            .into_iter()
            .map(procurement_inspection_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_procurement_inspection_detail(
    State(state): State<AppState>,
    Path((tenant_id, inspection_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let inspection_id = match uuid::Uuid::parse_str(&inspection_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad inspection_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_inspections.read") {
        return resp.into_response();
    }

    let repo = ProcurementInspectionsRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, inspection_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement inspection not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_inspection_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn create_procurement_inspection(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementInspectionCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_inspections.write") {
        return resp.into_response();
    }
    if req.items.is_empty() {
        return bad_request("items required").into_response();
    }

    let receipt_id = match req.receipt_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad receipt_id").into_response(),
        },
        None => None,
    };
    let supplier_id = match req.supplier_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let inspected_at = match parse_datetime_optional(req.inspected_at.as_deref(), "inspected_at") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let mut items = Vec::with_capacity(req.items.len());
    for item in req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(ProcurementInspectionItemInput {
            sku_id,
            qty: item.qty,
            passed_qty: item.passed_qty,
            failed_qty: item.failed_qty,
            result: normalize_optional_string(item.result),
        });
    }

    let repo = ProcurementInspectionsRepo::new(state.db.pool());
    let detail = match repo
        .create_inspection(ProcurementInspectionCreate {
            tenant_id,
            receipt_id,
            supplier_id,
            warehouse_id,
            inspected_at,
            remark: normalize_optional_string(req.remark),
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_inspection_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn update_procurement_inspection(
    State(state): State<AppState>,
    Path((tenant_id, inspection_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementInspectionUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let inspection_id = match uuid::Uuid::parse_str(&inspection_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad inspection_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_inspections.write") {
        return resp.into_response();
    }

    let inspected_at = match parse_datetime_optional(req.inspected_at.as_deref(), "inspected_at") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let items = match req.items {
        Some(items) => {
            if items.is_empty() {
                return bad_request("items required").into_response();
            }
            let mut converted = Vec::with_capacity(items.len());
            for item in items {
                let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
                    Ok(v) => v,
                    Err(_) => return bad_request("bad sku_id").into_response(),
                };
                converted.push(ProcurementInspectionItemInput {
                    sku_id,
                    qty: item.qty,
                    passed_qty: item.passed_qty,
                    failed_qty: item.failed_qty,
                    result: normalize_optional_string(item.result),
                });
            }
            Some(converted)
        }
        None => None,
    };
    let remark = normalize_optional_string(req.remark);
    if status.is_none() && inspected_at.is_none() && remark.is_none() && items.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let repo = ProcurementInspectionsRepo::new(state.db.pool());
    let detail = match repo
        .update_inspection(
            tenant_id,
            inspection_id,
            ProcurementInspectionUpdate {
                status,
                inspected_at,
                remark,
                items,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement inspection not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_inspection_detail_response(detail)),
    )
        .into_response()
}

pub(crate) async fn delete_procurement_inspection(
    State(state): State<AppState>,
    Path((tenant_id, inspection_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let inspection_id = match uuid::Uuid::parse_str(&inspection_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad inspection_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_inspections.write") {
        return resp.into_response();
    }

    let repo = ProcurementInspectionsRepo::new(state.db.pool());
    let inspection = match repo.delete_inspection(tenant_id, inspection_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement inspection not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_inspection_response(inspection)),
    )
        .into_response()
}

pub(crate) async fn list_procurement_reconciliations(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProcurementReconciliationListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_reconciliations.read") {
        return resp.into_response();
    }

    let supplier_id = match query.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let period_from = match parse_date_optional(query.period_from.as_deref(), "period_from") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let period_to = match parse_date_optional(query.period_to.as_deref(), "period_to") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = ProcurementReconciliationsRepo::new(state.db.pool());
    let result = match repo
        .list_reconciliations(
            tenant_id,
            ProcurementReconciliationListFilters {
                supplier_id,
                status,
                period_from,
                period_to,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ProcurementReconciliationListResponse {
        items: result
            .items
            .into_iter()
            .map(procurement_reconciliation_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_procurement_reconciliation(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementReconciliationCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_reconciliations.write") {
        return resp.into_response();
    }

    let supplier_id = match uuid::Uuid::parse_str(req.supplier_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad supplier_id").into_response(),
    };
    let period_start = match parse_date_optional(Some(req.period_start.as_str()), "period_start") {
        Ok(Some(v)) => v,
        Ok(None) => return bad_request("period_start required").into_response(),
        Err(resp) => return resp.into_response(),
    };
    let period_end = match parse_date_optional(Some(req.period_end.as_str()), "period_end") {
        Ok(Some(v)) => v,
        Ok(None) => return bad_request("period_end required").into_response(),
        Err(resp) => return resp.into_response(),
    };
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let repo = ProcurementReconciliationsRepo::new(state.db.pool());
    let reconciliation = match repo
        .create_reconciliation(ProcurementReconciliationCreate {
            tenant_id,
            supplier_id,
            period_start,
            period_end,
            amount_cent: req.amount_cent,
            status,
            remark: normalize_optional_string(req.remark),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_reconciliation_response(reconciliation)),
    )
        .into_response()
}

pub(crate) async fn update_procurement_reconciliation(
    State(state): State<AppState>,
    Path((tenant_id, reconciliation_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProcurementReconciliationUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let reconciliation_id = match uuid::Uuid::parse_str(&reconciliation_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad reconciliation_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_reconciliations.write") {
        return resp.into_response();
    }

    let supplier_id = match req.supplier_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let period_start = match parse_date_optional(req.period_start.as_deref(), "period_start") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let period_end = match parse_date_optional(req.period_end.as_deref(), "period_end") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let remark = normalize_optional_string(req.remark);
    if supplier_id.is_none()
        && period_start.is_none()
        && period_end.is_none()
        && req.amount_cent.is_none()
        && status.is_none()
        && remark.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let repo = ProcurementReconciliationsRepo::new(state.db.pool());
    let reconciliation = match repo
        .update_reconciliation(
            tenant_id,
            reconciliation_id,
            ProcurementReconciliationUpdate {
                supplier_id,
                period_start,
                period_end,
                amount_cent: req.amount_cent,
                status,
                remark,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement reconciliation not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_reconciliation_response(reconciliation)),
    )
        .into_response()
}

pub(crate) async fn delete_procurement_reconciliation(
    State(state): State<AppState>,
    Path((tenant_id, reconciliation_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let reconciliation_id = match uuid::Uuid::parse_str(&reconciliation_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad reconciliation_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "procurement_reconciliations.write") {
        return resp.into_response();
    }

    let repo = ProcurementReconciliationsRepo::new(state.db.pool());
    let reconciliation = match repo
        .delete_reconciliation(tenant_id, reconciliation_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("procurement reconciliation not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(procurement_reconciliation_response(reconciliation)),
    )
        .into_response()
}

pub(crate) async fn list_supplier_performance(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SupplierPerformanceListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "supplier_performances.read") {
        return resp.into_response();
    }

    let supplier_id = match query.supplier_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let period_from = match parse_date_optional(query.period_from.as_deref(), "period_from") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let period_to = match parse_date_optional(query.period_to.as_deref(), "period_to") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = SupplierPerformanceRepo::new(state.db.pool());
    let result = match repo
        .list_performance(
            tenant_id,
            SupplierPerformanceListFilters {
                supplier_id,
                status,
                period_from,
                period_to,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = SupplierPerformanceListResponse {
        items: result
            .items
            .into_iter()
            .map(supplier_performance_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_supplier_performance(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SupplierPerformanceCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "supplier_performances.write") {
        return resp.into_response();
    }

    let supplier_id = match uuid::Uuid::parse_str(req.supplier_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad supplier_id").into_response(),
    };
    let period_start = match parse_date_optional(Some(req.period_start.as_str()), "period_start") {
        Ok(Some(v)) => v,
        Ok(None) => return bad_request("period_start required").into_response(),
        Err(resp) => return resp.into_response(),
    };
    let period_end = match parse_date_optional(Some(req.period_end.as_str()), "period_end") {
        Ok(Some(v)) => v,
        Ok(None) => return bad_request("period_end required").into_response(),
        Err(resp) => return resp.into_response(),
    };
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let repo = SupplierPerformanceRepo::new(state.db.pool());
    let performance = match repo
        .create_performance(SupplierPerformanceCreate {
            tenant_id,
            supplier_id,
            period_start,
            period_end,
            delivery_score: req.delivery_score,
            quality_score: req.quality_score,
            price_score: req.price_score,
            status,
            remark: normalize_optional_string(req.remark),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(supplier_performance_response(performance)),
    )
        .into_response()
}

pub(crate) async fn update_supplier_performance(
    State(state): State<AppState>,
    Path((tenant_id, performance_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SupplierPerformanceUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let performance_id = match uuid::Uuid::parse_str(&performance_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad performance_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "supplier_performances.write") {
        return resp.into_response();
    }

    let supplier_id = match req.supplier_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let period_start = match parse_date_optional(req.period_start.as_deref(), "period_start") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let period_end = match parse_date_optional(req.period_end.as_deref(), "period_end") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let remark = normalize_optional_string(req.remark);
    if supplier_id.is_none()
        && period_start.is_none()
        && period_end.is_none()
        && req.delivery_score.is_none()
        && req.quality_score.is_none()
        && req.price_score.is_none()
        && status.is_none()
        && remark.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let repo = SupplierPerformanceRepo::new(state.db.pool());
    let performance = match repo
        .update_performance(
            tenant_id,
            performance_id,
            SupplierPerformanceUpdate {
                supplier_id,
                period_start,
                period_end,
                delivery_score: req.delivery_score,
                quality_score: req.quality_score,
                price_score: req.price_score,
                status,
                remark,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("supplier performance not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(supplier_performance_response(performance)),
    )
        .into_response()
}

pub(crate) async fn delete_supplier_performance(
    State(state): State<AppState>,
    Path((tenant_id, performance_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let performance_id = match uuid::Uuid::parse_str(&performance_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad performance_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "supplier_performances.write") {
        return resp.into_response();
    }

    let repo = SupplierPerformanceRepo::new(state.db.pool());
    let performance = match repo.delete_performance(tenant_id, performance_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("supplier performance not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(supplier_performance_response(performance)),
    )
        .into_response()
}
