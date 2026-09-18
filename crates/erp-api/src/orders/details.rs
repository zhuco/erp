use super::*;

pub(super) async fn list_orders(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OmsOrderListQuery>,
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

    let parse_csv = |raw: &str| {
        raw.split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
    };
    let platform_raw = query.platform.as_deref().and_then(non_empty);
    let (platform, platforms) = match platform_raw.as_deref() {
        Some(raw) => {
            let values = parse_csv(raw);
            if values.len() > 1 {
                let mut parsed = Vec::with_capacity(values.len());
                for value in values {
                    match Platform::from_str(value.as_str()) {
                        Ok(parsed_value) => parsed.push(parsed_value),
                        Err(_) => return bad_request("bad platform").into_response(),
                    }
                }
                (None, Some(parsed))
            } else {
                match Platform::from_str(values[0].as_str()) {
                    Ok(parsed_value) => (Some(parsed_value), None),
                    Err(_) => return bad_request("bad platform").into_response(),
                }
            }
        }
        None => (None, None),
    };
    let shop_id_raw = query.shop_id.as_deref().and_then(non_empty);
    let (shop_id, shop_ids_filter) = match shop_id_raw.as_deref() {
        Some(raw) => {
            let values = parse_csv(raw);
            if values.len() > 1 {
                let mut parsed = Vec::with_capacity(values.len());
                for value in values {
                    match ShopId::from_str(value.as_str()) {
                        Ok(parsed_value) => {
                            if let Err(resp) = ensure_shop_access(&ctx, parsed_value) {
                                return resp.into_response();
                            }
                            parsed.push(parsed_value);
                        }
                        Err(_) => return bad_request("bad shop_id").into_response(),
                    }
                }
                (None, Some(parsed))
            } else {
                match ShopId::from_str(values[0].as_str()) {
                    Ok(parsed_value) => {
                        if let Err(resp) = ensure_shop_access(&ctx, parsed_value) {
                            return resp.into_response();
                        }
                        (Some(parsed_value), None)
                    }
                    Err(_) => return bad_request("bad shop_id").into_response(),
                }
            }
        }
        None => (None, None),
    };
    let oms_status_raw = query.oms_status.as_deref().and_then(non_empty);
    let (oms_status, oms_statuses) = match oms_status_raw.as_deref() {
        Some(raw) => {
            let values = parse_csv(raw);
            if values.len() > 1 {
                let mut parsed = Vec::with_capacity(values.len());
                for value in values {
                    match OmsOrderStatus::from_str(value.as_str()) {
                        Ok(parsed_value) => parsed.push(parsed_value),
                        Err(_) => return bad_request("bad oms_status").into_response(),
                    }
                }
                (None, Some(parsed))
            } else {
                match OmsOrderStatus::from_str(values[0].as_str()) {
                    Ok(parsed_value) => (Some(parsed_value), None),
                    Err(_) => return bad_request("bad oms_status").into_response(),
                }
            }
        }
        None => (None, None),
    };
    let order_status = match query.order_status.as_deref() {
        Some(raw) => match OrderMainStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad order_status").into_response(),
        },
        None => None,
    };
    let review_status = match query.review_status.as_deref() {
        Some(raw) => match OrderReviewStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad review_status").into_response(),
        },
        None => None,
    };
    let fulfillment_status = match query.fulfillment_status.as_deref() {
        Some(raw) => match OrderFulfillmentStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad fulfillment_status").into_response(),
        },
        None => None,
    };
    let exception_status = match query.exception_status.as_deref() {
        Some(raw) => match OrderExceptionStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad exception_status").into_response(),
        },
        None => None,
    };
    let payment_status = match query.payment_status.as_deref() {
        Some(raw) => match OrderPaymentStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad payment_status").into_response(),
        },
        None => None,
    };
    let exception_flags = query
        .exception_flags
        .as_deref()
        .map(|raw| {
            raw.split(',')
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    let tags = query
        .tags
        .as_deref()
        .map(|raw| {
            raw.split(',')
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    if !tags.is_empty() {
        let enabled =
            match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
                Ok(v) => v,
                Err(resp) => return resp.into_response(),
            };
        if !enabled {
            return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
        }
    }

    let parse_dt = |raw: &str, label: &str| -> Result<DateTime<Utc>, Response> {
        DateTime::parse_from_rfc3339(raw)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|_| bad_request(&format!("bad {label}")).into_response())
    };
    let created_from = match query.created_from.as_deref() {
        Some(raw) => match parse_dt(raw, "created_from") {
            Ok(v) => Some(v),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let created_to = match query.created_to.as_deref() {
        Some(raw) => match parse_dt(raw, "created_to") {
            Ok(v) => Some(v),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let updated_from = match query.updated_from.as_deref() {
        Some(raw) => match parse_dt(raw, "updated_from") {
            Ok(v) => Some(v),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let updated_to = match query.updated_to.as_deref() {
        Some(raw) => match parse_dt(raw, "updated_to") {
            Ok(v) => Some(v),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let parse_i64 = |raw: &str, label: &str| -> Result<i64, Response> {
        let value = raw
            .parse::<i64>()
            .map_err(|_| bad_request(&format!("bad {label}")).into_response())?;
        if value < 0 {
            return Err(bad_request(&format!("{label} must be non-negative")).into_response());
        }
        Ok(value)
    };
    let parse_f64 = |raw: &str, label: &str| -> Result<f64, Response> {
        let value = raw
            .parse::<f64>()
            .map_err(|_| bad_request(&format!("bad {label}")).into_response())?;
        if value < 0.0 {
            return Err(bad_request(&format!("{label} must be non-negative")).into_response());
        }
        Ok(value)
    };
    let ship_warehouse_raw = query.ship_warehouse_id.as_deref().and_then(non_empty);
    let (ship_warehouse_id, ship_warehouse_ids) = match ship_warehouse_raw.as_deref() {
        Some(raw) => {
            let values = parse_csv(raw);
            if values.len() > 1 {
                let mut parsed = Vec::with_capacity(values.len());
                for value in values {
                    match uuid::Uuid::parse_str(value.as_str()) {
                        Ok(parsed_value) => parsed.push(parsed_value),
                        Err(_) => return bad_request("bad ship_warehouse_id").into_response(),
                    }
                }
                (None, Some(parsed))
            } else {
                match uuid::Uuid::parse_str(values[0].as_str()) {
                    Ok(parsed_value) => (Some(parsed_value), None),
                    Err(_) => return bad_request("bad ship_warehouse_id").into_response(),
                }
            }
        }
        None => (None, None),
    };
    let platform_order_id = query.platform_order_id.as_deref().and_then(non_empty);
    let item_keyword = query.item_keyword.as_deref().and_then(non_empty);
    let total_amount_cent = match query.total_amount_cent.as_deref().and_then(non_empty) {
        Some(raw) => match parse_i64(raw.as_str(), "total_amount_cent") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let total_amount_min_cent = match query
        .total_amount_min_cent
        .as_deref()
        .and_then(non_empty)
    {
        Some(raw) => match parse_i64(raw.as_str(), "total_amount_min_cent") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let total_amount_max_cent = match query
        .total_amount_max_cent
        .as_deref()
        .and_then(non_empty)
    {
        Some(raw) => match parse_i64(raw.as_str(), "total_amount_max_cent") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    if let Err(message) = validate_total_amount_range(total_amount_min_cent, total_amount_max_cent) {
        return bad_request(message).into_response();
    }
    let goods_qty = match query.goods_qty.as_deref().and_then(non_empty) {
        Some(raw) => match parse_i64(raw.as_str(), "goods_qty") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let customer_account = query.customer_account.as_deref().and_then(non_empty);
    let receiver_name = query.receiver_name.as_deref().and_then(non_empty);
    let receiver_phone = query.receiver_phone.as_deref().and_then(non_empty);
    let receiver_address = query.receiver_address.as_deref().and_then(non_empty);
    let remark = query.remark.as_deref().and_then(non_empty);
    let order_source = query.order_source.as_deref().and_then(non_empty);
    let logistics_company = query.logistics_company.as_deref().and_then(non_empty);
    let estimated_postage_cent = match query.estimated_postage_cent.as_deref().and_then(non_empty) {
        Some(raw) => match parse_i64(raw.as_str(), "estimated_postage_cent") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let estimated_profit_cent = match query.estimated_profit_cent.as_deref().and_then(non_empty) {
        Some(raw) => match parse_i64(raw.as_str(), "estimated_profit_cent") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let estimated_profit_rate = match query.estimated_profit_rate.as_deref().and_then(non_empty) {
        Some(raw) => match parse_f64(raw.as_str(), "estimated_profit_rate") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let filters = OmsOrderListFilters {
        platform,
        platforms,
        shop_id,
        shop_ids: shop_scope_filter(&ctx),
        shop_ids_filter,
        oms_status,
        oms_statuses,
        order_status,
        review_status,
        fulfillment_status,
        exception_status,
        payment_status,
        exception_flags,
        tags,
        tagged: query.tagged,
        created_from,
        created_to,
        updated_from,
        updated_to,
        ship_warehouse_id,
        ship_warehouse_ids,
        platform_order_id,
        item_keyword,
        total_amount_cent,
        total_amount_min_cent,
        total_amount_max_cent,
        goods_qty,
        customer_account,
        receiver_name,
        receiver_phone,
        receiver_address,
        remark,
        order_source,
        logistics_company,
        estimated_postage_cent,
        estimated_profit_cent,
        estimated_profit_rate,
        keyword: query.keyword,
        offset,
        limit,
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    match orders_repo.list_orders(tenant_id, filters).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

fn validate_total_amount_range(min: Option<i64>, max: Option<i64>) -> Result<(), &'static str> {
    if min.is_some_and(|value| value < 0) || max.is_some_and(|value| value < 0) {
        return Err("total amount range must be non-negative");
    }
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        return Err("total amount minimum exceeds maximum");
    }
    Ok(())
}

#[cfg(test)]
mod amount_range_tests {
    use super::validate_total_amount_range;

    #[test]
    fn amount_range_accepts_open_and_equal_bounds_and_rejects_invalid_ranges() {
        assert_eq!(validate_total_amount_range(None, None), Ok(()));
        assert_eq!(validate_total_amount_range(Some(0), None), Ok(()));
        assert_eq!(validate_total_amount_range(Some(500), Some(500)), Ok(()));
        assert!(validate_total_amount_range(Some(-1), None).is_err());
        assert!(validate_total_amount_range(Some(501), Some(500)).is_err());
    }
}

pub(super) async fn get_order_detail(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
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
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
        return resp.into_response();
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let audit_logs = match audit_repo
        .list_logs(
            tenant_id,
            erp_repo::audit_logs::AuditLogListFilters {
                action: None,
                entity_type: Some("ORDER".to_string()),
                entity_id: Some(order_id.to_string()),
                user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit: 50,
            },
        )
        .await
    {
        Ok(result) => result.items,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = OmsOrderDetailResponse {
        order: detail.order,
        items: detail.items,
        review_hits: detail.review_hits,
        tags: detail.tags,
        notes: detail.notes,
        audit_logs,
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn update_order_receiver(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderReceiverUpdateRequest>,
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
    let receiver_name = req.receiver_name.trim();
    if receiver_name.is_empty() {
        return bad_request("receiver_name required").into_response();
    }
    let receiver_phone = req.receiver_phone.trim();
    if receiver_phone.is_empty() {
        return bad_request("receiver_phone required").into_response();
    }
    let receiver_address = req.receiver_address.trim();
    if receiver_address.is_empty() {
        return bad_request("receiver_address required").into_response();
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

    if matches!(
        detail.order.oms_status,
        OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
    ) {
        return bad_request("order already shipped").into_response();
    }
    if detail.order.review_status.is_approved() {
        return bad_request("reviewed order only allows logistics update").into_response();
    }

    let before_json = serde_json::json!({
        "receiver_name": detail.order.receiver_name,
        "receiver_phone": detail.order.receiver_phone,
        "receiver_address": detail.order.receiver_address,
    });

    let updated = match orders_repo
        .update_order_receiver(
            tenant_id,
            order_id,
            OrderReceiverUpdate {
                receiver_name: receiver_name.to_string(),
                receiver_phone: receiver_phone.to_string(),
                receiver_address: receiver_address.to_string(),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if !updated {
        return not_found("order not found").into_response();
    }

    let updated_detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.ORDER_RECEIVER_UPDATE".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: Some(before_json),
            after_json: Some(serde_json::json!({
                "receiver_name": receiver_name,
                "receiver_phone": receiver_phone,
                "receiver_address": receiver_address,
            })),
            ip: None,
            ua: None,
        })
        .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "OMS.ORDER_RECEIVER_UPDATE".to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "receiver_name": receiver_name,
                "receiver_phone": receiver_phone,
                "receiver_address": receiver_address,
            })),
        })
        .await;

    let audit_logs = match audit_repo
        .list_logs(
            tenant_id,
            erp_repo::audit_logs::AuditLogListFilters {
                action: None,
                entity_type: Some("ORDER".to_string()),
                entity_id: Some(order_id.to_string()),
                user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit: 50,
            },
        )
        .await
    {
        Ok(result) => result.items,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = OmsOrderDetailResponse {
        order: updated_detail.order,
        items: updated_detail.items,
        review_hits: updated_detail.review_hits,
        tags: updated_detail.tags,
        notes: updated_detail.notes,
        audit_logs,
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn edit_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderEditRequest>,
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

    let orders_repo = OrdersRepo::new(state.db.pool());
    let before_detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, before_detail.order.shop_id) {
        return resp.into_response();
    }

    if matches!(
        before_detail.order.oms_status,
        OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
    ) {
        return bad_request("order already shipped").into_response();
    }

    let receiver_name = req.receiver_name.and_then(non_empty);
    let receiver_phone = req.receiver_phone.and_then(non_empty);
    let receiver_tel = req.receiver_tel.and_then(non_empty);
    let receiver_address = req.receiver_address.and_then(non_empty);
    let logistics_company = req.logistics_company.and_then(non_empty);
    let tracking_no = req.tracking_no.and_then(non_empty);
    let order_type = req.order_type.and_then(non_empty);
    let sales_channel = req.sales_channel.and_then(non_empty);
    let order_source = req.order_source.and_then(non_empty);
    let salesperson = req.salesperson.and_then(non_empty);
    let ship_warehouse_id = match req.ship_warehouse_id.as_deref().and_then(non_empty) {
        Some(raw) => match uuid::Uuid::parse_str(raw.as_str()) {
            Ok(value) => Some(value),
            Err(_) => return bad_request("bad ship_warehouse_id").into_response(),
        },
        None => None,
    };
    let shipment_no = req.shipment_no.and_then(non_empty);
    let estimated_weight_gram = req.estimated_weight_gram;
    if let Some(value) = estimated_weight_gram {
        if value < 0 {
            return bad_request("estimated_weight_gram must be non-negative").into_response();
        }
    }
    let package_weight_gram = req.package_weight_gram;
    if let Some(value) = package_weight_gram {
        if value < 0 {
            return bad_request("package_weight_gram must be non-negative").into_response();
        }
    }
    let customer_account = req.customer_account.and_then(non_empty);
    let paid_amount_cent = req.paid_amount_cent;
    if let Some(value) = paid_amount_cent {
        if value < 0 {
            return bad_request("paid_amount_cent must be non-negative").into_response();
        }
    }
    let subsidy_amount_cent = req.subsidy_amount_cent;
    if let Some(value) = subsidy_amount_cent {
        if value < 0 {
            return bad_request("subsidy_amount_cent must be non-negative").into_response();
        }
    }
    let receivable_amount_cent = req.receivable_amount_cent;
    if let Some(value) = receivable_amount_cent {
        if value < 0 {
            return bad_request("receivable_amount_cent must be non-negative").into_response();
        }
    }
    let other_fee_cent = req.other_fee_cent;
    if let Some(value) = other_fee_cent {
        if value < 0 {
            return bad_request("other_fee_cent must be non-negative").into_response();
        }
    }
    let estimated_postage_cent = req.estimated_postage_cent;
    if let Some(value) = estimated_postage_cent {
        if value < 0 {
            return bad_request("estimated_postage_cent must be non-negative").into_response();
        }
    }
    let invoice_type = req.invoice_type.and_then(non_empty);
    let invoice_title = req.invoice_title.and_then(non_empty);
    let payment_status = req.payment_status.and_then(non_empty);
    let buyer_message = req.buyer_message.and_then(non_empty);
    let seller_remark = req.seller_remark.and_then(non_empty);
    let freeze_reason = req.freeze_reason.and_then(non_empty);
    let cancel_reason = req.cancel_reason.and_then(non_empty);
    let exception_reason = req.exception_reason.and_then(non_empty);
    let extra_remark = req.extra_remark.and_then(non_empty);

    let item_inputs = match req.items {
        Some(items) => {
            if req.expected_version.is_none() {
                return bad_request("expected_version required when items are provided")
                    .into_response();
            }
            if items.is_empty() {
                return bad_request("items required").into_response();
            }
            let mut mapped = Vec::with_capacity(items.len());
            for item in items {
                if item.qty <= 0 {
                    return bad_request("item qty must be positive").into_response();
                }
                if item.price_cent < 0 {
                    return bad_request("item price_cent must be non-negative").into_response();
                }
                mapped.push(OrderEditItemInput {
                    line_id: item.line_id.and_then(non_empty),
                    sku_id: item.sku_id.and_then(non_empty),
                    title: item.title.and_then(non_empty),
                    qty: item.qty,
                    price_cent: item.price_cent,
                });
            }
            Some(mapped)
        }
        None => None,
    };
    if item_inputs.is_some() && req.item_edits.is_some() {
        return bad_request("items and item_edits are mutually exclusive").into_response();
    }
    let item_edits = match req.item_edits {
        Some(edits) => {
            if req.expected_version.is_none() {
                return bad_request("expected_version required when item_edits are provided")
                    .into_response();
            }
            if edits.is_empty() {
                return bad_request("item_edits required").into_response();
            }
            let mut line_ids = std::collections::BTreeSet::new();
            let mut mapped = Vec::with_capacity(edits.len());
            for edit in edits {
                let line_id = edit.line_id;
                if line_id.trim().is_empty() {
                    return bad_request("item_edit line_id required").into_response();
                }
                if !line_ids.insert(line_id.clone()) {
                    return bad_request("duplicate item_edit line_id").into_response();
                }
                if edit.qty.is_none() && edit.price_cent.is_none() {
                    return bad_request("item_edit qty or price_cent required").into_response();
                }
                if edit.qty.is_some_and(|qty| qty <= 0) {
                    return bad_request("item_edit qty must be positive").into_response();
                }
                if edit.price_cent.is_some_and(|price| price < 0) {
                    return bad_request("item_edit price_cent must be non-negative")
                        .into_response();
                }
                mapped.push(erp_repo::orders::OrderItemEditInput {
                    line_id,
                    qty: edit.qty,
                    price_cent: edit.price_cent,
                });
            }
            Some(mapped)
        }
        None => None,
    };

    let has_detail_change = receiver_name.is_some()
        || receiver_phone.is_some()
        || receiver_tel.is_some()
        || receiver_address.is_some()
        || order_type.is_some()
        || sales_channel.is_some()
        || order_source.is_some()
        || salesperson.is_some()
        || customer_account.is_some()
        || paid_amount_cent.is_some()
        || subsidy_amount_cent.is_some()
        || receivable_amount_cent.is_some()
        || other_fee_cent.is_some()
        || estimated_postage_cent.is_some()
        || invoice_type.is_some()
        || invoice_title.is_some()
        || payment_status.is_some()
        || buyer_message.is_some()
        || seller_remark.is_some()
        || freeze_reason.is_some()
        || cancel_reason.is_some()
        || exception_reason.is_some()
        || extra_remark.is_some()
        || item_inputs.as_ref().is_some_and(|items| !items.is_empty())
        || item_edits.as_ref().is_some_and(|items| !items.is_empty());
    let has_logistics_change = logistics_company.is_some()
        || tracking_no.is_some()
        || ship_warehouse_id.is_some()
        || shipment_no.is_some()
        || estimated_weight_gram.is_some()
        || package_weight_gram.is_some();

    if !has_detail_change && !has_logistics_change {
        return bad_request("no fields to update").into_response();
    }
    if before_detail.order.review_status.is_approved() && has_detail_change {
        return bad_request("reviewed order only allows logistics update").into_response();
    }

    let log_items = item_inputs.clone();
    let log_item_edits_json = item_edits.as_ref().map(|edits| {
        edits
            .iter()
            .map(|edit| serde_json::json!({
                "line_id": edit.line_id,
                "qty": edit.qty,
                "price_cent": edit.price_cent,
            }))
            .collect::<Vec<_>>()
    });
    let log_items_json = log_items.as_ref().map(|items| {
        items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "line_id": item.line_id,
                    "sku_id": item.sku_id,
                    "title": item.title,
                    "qty": item.qty,
                    "price_cent": item.price_cent,
                })
            })
            .collect::<Vec<_>>()
    });
    let edit_input = OrderEditInput {
        expected_version: req.expected_version,
        receiver_name: receiver_name.clone(),
        receiver_phone: receiver_phone.clone(),
        receiver_tel: receiver_tel.clone(),
        receiver_address: receiver_address.clone(),
        logistics_company: logistics_company.clone(),
        tracking_no: tracking_no.clone(),
        order_type: order_type.clone(),
        sales_channel: sales_channel.clone(),
        order_source: order_source.clone(),
        salesperson: salesperson.clone(),
        ship_warehouse_id,
        shipment_no: shipment_no.clone(),
        estimated_weight_gram,
        package_weight_gram,
        customer_account: customer_account.clone(),
        paid_amount_cent,
        subsidy_amount_cent,
        receivable_amount_cent,
        other_fee_cent,
        estimated_postage_cent,
        invoice_type: invoice_type.clone(),
        invoice_title: invoice_title.clone(),
        payment_status: payment_status.clone(),
        buyer_message: buyer_message.clone(),
        seller_remark: seller_remark.clone(),
        freeze_reason: freeze_reason.clone(),
        cancel_reason: cancel_reason.clone(),
        exception_reason: exception_reason.clone(),
        extra_remark: extra_remark.clone(),
        items: item_inputs,
        item_edits,
    };

    let updated = match orders_repo
        .update_order_edit(tenant_id, order_id, edit_input)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    match updated {
        erp_repo::orders::OrderEditOutcome::Updated => {}
        erp_repo::orders::OrderEditOutcome::NotFound => {
            return not_found("order not found").into_response();
        }
        erp_repo::orders::OrderEditOutcome::MissingExpectedVersion => {
            return bad_request("expected_version required when items are provided")
                .into_response();
        }
        erp_repo::orders::OrderEditOutcome::VersionConflict => {
            return conflict("order version conflict").into_response();
        }
        erp_repo::orders::OrderEditOutcome::InvalidItemEdits => {
            return bad_request("invalid item_edits").into_response();
        }
    }

    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let before_items: Vec<serde_json::Value> = before_detail
        .items
        .iter()
        .map(|item| {
            serde_json::json!({
                "id": item.id,
                "platform_sku_id": item.platform_sku_id,
                "outer_sku_id": item.outer_sku_id,
                "title": item.title,
                "quantity": item.quantity,
                "price_cent": item.price_cent,
            })
        })
        .collect();

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.ORDER_EDIT".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: Some(serde_json::json!({
                "receiver_name": before_detail.order.receiver_name,
                "receiver_phone": before_detail.order.receiver_phone,
                "receiver_tel": before_detail.order.receiver_tel,
                "receiver_address": before_detail.order.receiver_address,
                "logistics_company": before_detail.order.logistics_company,
                "tracking_no": before_detail.order.tracking_no,
                "order_type": before_detail.order.order_type,
                "sales_channel": before_detail.order.sales_channel,
                "order_source": before_detail.order.order_source,
                "salesperson": before_detail.order.salesperson,
                "ship_warehouse_id": before_detail.order.ship_warehouse_id,
                "shipment_no": before_detail.order.shipment_no,
                "estimated_weight_gram": before_detail.order.estimated_weight_gram,
                "package_weight_gram": before_detail.order.package_weight_gram,
                "customer_account": before_detail.order.customer_account,
                "paid_amount_cent": before_detail.order.paid_amount_cent,
                "subsidy_amount_cent": before_detail.order.subsidy_amount_cent,
                "receivable_amount_cent": before_detail.order.receivable_amount_cent,
                "other_fee_cent": before_detail.order.other_fee_cent,
                "estimated_postage_cent": before_detail.order.estimated_postage_cent,
                "invoice_type": before_detail.order.invoice_type,
                "invoice_title": before_detail.order.invoice_title,
                "payment_status": before_detail.order.payment_status,
                "buyer_message": before_detail.order.buyer_message,
                "seller_remark": before_detail.order.seller_remark,
                "freeze_reason": before_detail.order.freeze_reason,
                "cancel_reason": before_detail.order.cancel_reason,
                "exception_reason": before_detail.order.exception_reason,
                "extra_remark": before_detail.order.extra_remark,
                "items": before_items,
            })),
            after_json: Some(serde_json::json!({
                "receiver_name": receiver_name,
                "receiver_phone": receiver_phone,
                "receiver_tel": receiver_tel,
                "receiver_address": receiver_address,
                "logistics_company": logistics_company,
                "tracking_no": tracking_no,
                "order_type": order_type,
                "sales_channel": sales_channel,
                "order_source": order_source,
                "salesperson": salesperson,
                "ship_warehouse_id": ship_warehouse_id,
                "shipment_no": shipment_no,
                "estimated_weight_gram": estimated_weight_gram,
                "package_weight_gram": package_weight_gram,
                "customer_account": customer_account,
                "paid_amount_cent": paid_amount_cent,
                "subsidy_amount_cent": subsidy_amount_cent,
                "receivable_amount_cent": receivable_amount_cent,
                "other_fee_cent": other_fee_cent,
                "estimated_postage_cent": estimated_postage_cent,
                "invoice_type": invoice_type,
                "invoice_title": invoice_title,
                "payment_status": payment_status,
                "buyer_message": buyer_message,
                "seller_remark": seller_remark,
                "freeze_reason": freeze_reason,
                "cancel_reason": cancel_reason,
                "exception_reason": exception_reason,
                "extra_remark": extra_remark,
                "items": log_items_json,
                "item_edits": log_item_edits_json,
                "expected_version": req.expected_version,
                "readback_version": detail.order.version,
            })),
            ip: None,
            ua: None,
        })
        .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "OMS.ORDER_EDIT".to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "receiver_name": receiver_name,
                "receiver_phone": receiver_phone,
                "receiver_address": receiver_address,
                "logistics_company": logistics_company,
                "tracking_no": tracking_no,
                "shipment_no": shipment_no,
                "items": log_items_json,
                "item_edits": log_item_edits_json,
                "expected_version": req.expected_version,
                "readback_version": detail.order.version,
            })),
        })
        .await;

    let audit_logs = match audit_repo
        .list_logs(
            tenant_id,
            erp_repo::audit_logs::AuditLogListFilters {
                action: None,
                entity_type: Some("ORDER".to_string()),
                entity_id: Some(order_id.to_string()),
                user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit: 50,
            },
        )
        .await
    {
        Ok(v) => v.items,
        Err(_) => vec![],
    };

    let response = OmsOrderDetailResponse {
        order: detail.order,
        items: detail.items,
        review_hits: detail.review_hits,
        tags: detail.tags,
        notes: detail.notes,
        audit_logs,
    };

    (StatusCode::OK, Json(response)).into_response()
}
