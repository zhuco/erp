use super::*;
use erp_repo::{
    operation_logs::{
        OP_ACTION_OMS_ORDER_MERGE_SOURCE, OP_ACTION_OMS_ORDER_MERGE_TARGET,
        OP_ACTION_OMS_ORDER_SPLIT_CHILD, OP_ACTION_OMS_ORDER_SPLIT_SOURCE,
    },
    orders::{OrderSplitPlanChild, OrderSplitPlanItem},
};
pub(super) async fn get_order_chain(
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
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.read") {
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
                limit: 100,
            },
        )
        .await
    {
        Ok(result) => result.items,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let order_response = OmsOrderDetailResponse {
        order: detail.order,
        items: detail.items,
        review_hits: detail.review_hits,
        tags: detail.tags,
        notes: detail.notes,
        audit_logs,
    };

    let fulfillment_repo = FulfillmentRepo::new(state.db.pool());
    let shipment_result = match fulfillment_repo
        .list_oms_shipments(
            tenant_id,
            OmsShipmentListFilters {
                order_id: Some(uuid::Uuid::from(order_id)),
                status: None,
                warehouse_id: None,
                shipment_types: vec![
                    SHIPMENT_TYPE_OMS.to_string(),
                    SHIPMENT_TYPE_EXCHANGE.to_string(),
                    SHIPMENT_TYPE_RESEND.to_string(),
                ],
                limit: 200,
                offset: 0,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let mut shipments = Vec::new();
    for shipment in shipment_result.items {
        let shipment_items =
            match load_oms_shipment_items(state.db.pool(), tenant_id, shipment.shipment_id).await {
                Ok(v) => v,
                Err(resp) => return resp,
            };
        shipments.push(oms_shipment_summary(
            uuid::Uuid::from(tenant_id),
            shipment,
            Some(shipment_items),
        ));
    }

    let after_sales_repo = AfterSalesRepo::new(state.db.pool());
    let after_sales_list = match after_sales_repo
        .list_after_sales(
            tenant_id,
            AfterSalesListFilters {
                status: None,
                after_sales_type: None,
                order_id: Some(order_id),
                created_from: None,
                created_to: None,
                offset: 0,
                limit: 200,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut after_sales = Vec::new();
    let mut after_sales_ids = Vec::new();
    for entry in after_sales_list.items {
        let detail = match after_sales_repo.get_detail(tenant_id, entry.id).await {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        let audit_logs = match load_after_sales_audit_logs(
            state.db.pool(),
            tenant_id,
            detail.after_sales.id,
            detail.refund_doc.as_ref().map(|doc| doc.id),
            100,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        after_sales_ids.push(detail.after_sales.id);
        after_sales.push(after_sales_detail_view(detail, audit_logs));
    }

    let inbounds_repo = InboundsRepo::new(state.db.pool());
    let mut inbounds = Vec::new();
    let mut seen_doc_ids = HashSet::new();
    for after_sales_id in after_sales_ids {
        let docs = match inbounds_repo
            .list_docs(
                tenant_id,
                InboundDocListFilters {
                    status: None,
                    inbound_status: None,
                    receipt_status: None,
                    inbound_qc_status: None,
                    putaway_status: None,
                    doc_type: None,
                    warehouse_id: None,
                    doc_no: None,
                    doc_no_keyword: None,
                    ref_type: Some("AFTER_SALES".to_string()),
                    ref_id: Some(after_sales_id.to_string()),
                    supplier_id: None,
                    sku_id: None,
                    created_from: None,
                    created_to: None,
                    offset: 0,
                    limit: 200,
                },
            )
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        for doc in docs.items {
            if seen_doc_ids.insert(doc.doc_id) {
                inbounds.push(inbound_doc_summary_response(doc));
            }
        }
    }

    inbounds.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| b.doc_id.cmp(&a.doc_id))
    });

    let response = OmsOrderChainResponse {
        order: order_response,
        shipments,
        after_sales,
        inbounds,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn merge_orders(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderMergeRequest>,
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

    let OrderMergeRequest {
        order_ids: raw_order_ids,
        target_order_id,
        force,
    } = req;
    let force_merge = force.unwrap_or(false);

    let mut order_ids = raw_order_ids
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    order_ids.sort();
    order_ids.dedup();
    if order_ids.len() < 2 {
        return bad_request("order_ids required").into_response();
    }

    let parsed_ids = order_ids
        .iter()
        .map(|raw| OrderId::from_str(raw))
        .collect::<Result<Vec<_>, _>>();
    let order_ids = match parsed_ids {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    if let Some(target_raw) = target_order_id.as_deref() {
        if !order_ids.iter().any(|id| id.to_string() == target_raw) {
            return bad_request("target_order_id not in order_ids").into_response();
        }
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let mut details = Vec::with_capacity(order_ids.len());
    for order_id in &order_ids {
        let detail = match orders_repo.get_order_detail(tenant_id, *order_id).await {
            Ok(Some(v)) => v,
            Ok(None) => return not_found("order not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
            return resp.into_response();
        }
        details.push(detail);
    }

    let mut identity_key: Option<(String, String, String, uuid::Uuid)> = None;
    for detail in &details {
        let order = &detail.order;
        if !order.review_status.is_pending() {
            return bad_request("order review_status not pending").into_response();
        }
        if matches!(
            order.oms_status,
            OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
        ) {
            return bad_request("order status not mergeable").into_response();
        }
        let customer_key = order
            .customer_id
            .map(|value| value.to_string())
            .or_else(|| order.customer_name.clone())
            .unwrap_or_default();
        let phone_key = order.receiver_phone.clone().unwrap_or_default();
        let address_key = order.receiver_address.clone().unwrap_or_default();
        let shop_id = uuid::Uuid::from(order.shop_id);
        if let Some((customer, phone, address, shop)) = &identity_key {
            if (*customer != customer_key || *phone != phone_key || *address != address_key)
                && !force_merge
            {
                return bad_request("merge condition mismatch").into_response();
            }
            if *shop != shop_id {
                return bad_request("orders from different shops").into_response();
            }
        } else {
            identity_key = Some((customer_key, phone_key, address_key, shop_id));
        }
    }

    let target_order_id = if let Some(target_raw) = target_order_id.as_deref() {
        OrderId::from_str(target_raw).unwrap_or(order_ids[0])
    } else {
        details
            .iter()
            .min_by_key(|detail| detail.order.order_created_at)
            .map(|detail| detail.order.id)
            .unwrap_or(order_ids[0])
    };

    let mut merged_items = Vec::new();
    for order_id in &order_ids {
        let items = match orders_repo
            .list_order_edit_items(tenant_id, *order_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        for item in items {
            merged_items.push(OrderEditItemInput {
                line_id: None,
                sku_id: item.sku_id,
                title: item.title,
                qty: item.qty,
                price_cent: item.price_cent,
            });
        }
    }

    if merged_items.is_empty() {
        return bad_request("merge items empty").into_response();
    }

    let update_result = orders_repo
        .update_order_edit(
            tenant_id,
            target_order_id,
            OrderEditInput {
                receiver_name: None,
                receiver_phone: None,
                receiver_tel: None,
                receiver_address: None,
                logistics_company: None,
                tracking_no: None,
                order_type: None,
                sales_channel: None,
                order_source: None,
                salesperson: None,
                ship_warehouse_id: None,
                shipment_no: None,
                estimated_weight_gram: None,
                package_weight_gram: None,
                customer_account: None,
                paid_amount_cent: None,
                subsidy_amount_cent: None,
                receivable_amount_cent: None,
                other_fee_cent: None,
                estimated_postage_cent: None,
                invoice_type: None,
                invoice_title: None,
                payment_status: None,
                buyer_message: None,
                seller_remark: None,
                freeze_reason: None,
                cancel_reason: None,
                exception_reason: None,
                extra_remark: None,
                items: Some(merged_items),
            },
        )
        .await;
    match update_result {
        Ok(Some(())) => {}
        Ok(None) => return not_found("target order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }

    let total_amount_cent = details
        .iter()
        .map(|detail| detail.order.total_amount_cent)
        .sum::<i64>();
    let sum_optional = |values: Vec<Option<i64>>| -> Option<i64> {
        let mut sum = 0;
        let mut has_value = false;
        for v in values.into_iter().flatten() {
            sum += v;
            has_value = true;
        }
        if has_value {
            Some(sum)
        } else {
            None
        }
    };
    let goods_amount_cent = sum_optional(
        details
            .iter()
            .map(|detail| detail.order.goods_amount_cent)
            .collect(),
    );
    let postage_amount_cent = sum_optional(
        details
            .iter()
            .map(|detail| detail.order.postage_amount_cent)
            .collect(),
    );
    let discount_amount_cent = sum_optional(
        details
            .iter()
            .map(|detail| detail.order.discount_amount_cent)
            .collect(),
    );
    let _ = orders_repo
        .update_order_amounts(
            tenant_id,
            target_order_id,
            total_amount_cent,
            goods_amount_cent,
            postage_amount_cent,
            discount_amount_cent,
        )
        .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let mut merged_order_ids = Vec::new();
    for detail in &details {
        if detail.order.id == target_order_id {
            continue;
        }
        let state_row = match orders_repo
            .get_order_review_state(tenant_id, detail.order.id)
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        let mut next_flags = state_row.exception_flags.clone();
        if !next_flags.iter().any(|flag| flag == ORDER_MERGED_FLAG) {
            next_flags.push(ORDER_MERGED_FLAG.to_string());
        }
        let exception_status = OrderExceptionStatus::Closed;
        let order_status = OrderMainStatus::Cancelled;
        let updated = orders_repo
            .update_order_review_state(
                tenant_id,
                detail.order.id,
                OrderReviewUpdate {
                    oms_status: OmsOrderStatus::Cancelled,
                    order_status,
                    review_status: OrderReviewStatus::Rejected,
                    fulfillment_status: state_row.fulfillment_status,
                    exception_status,
                    exception_flags: next_flags.clone(),
                    expected_version: Some(state_row.version),
                },
            )
            .await;
        match updated {
            Ok(Some(_)) => {}
            Ok(None) => return conflict("merge update conflict").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }

        upsert_order_review_record(
            &state,
            tenant_id,
            detail.order.id,
            OrderReviewStatus::Rejected,
            Some(ctx.user_id),
            Some(Utc::now()),
            None,
        )
        .await;

        record_order_state_logs(
            &state,
            &req_ctx,
            OrderStateLogInput {
                tenant_id,
                order_id: detail.order.id,
                action_code: "OMS.ORDER_MERGE_SOURCE".to_string(),
                from_status: state_row.order_status,
                to_status: order_status,
                operator_id: Some(ctx.user_id),
                reason: None,
                request_payload: serde_json::json!({
                    "target_order_id": target_order_id.to_string(),
                    "force": force_merge,
                }),
                response_payload: serde_json::json!({
                    "oms_status": OmsOrderStatus::Cancelled.to_string(),
                    "order_status": order_status.to_string(),
                    "review_status": OrderReviewStatus::Rejected.to_string(),
                    "exception_flags": next_flags.clone(),
                }),
            },
        )
        .await;
        merged_order_ids.push(detail.order.id.to_string());

        record_audit_log(
            &state,
            &ctx,
            &req_ctx,
            "OMS.ORDER_MERGE_SOURCE",
            "ORDER",
            Some(detail.order.id.to_string()),
            Some(serde_json::json!({
                "oms_status": state_row.oms_status,
                "review_status": state_row.review_status,
                "exception_flags": state_row.exception_flags,
            })),
            Some(serde_json::json!({
                "oms_status": OmsOrderStatus::Cancelled.to_string(),
                "review_status": OrderReviewStatus::Rejected.to_string(),
                "exception_flags": next_flags,
                "target_order_id": target_order_id.to_string(),
            })),
        )
        .await;

        let _ = logs_repo
            .insert(OperationLogCreate {
                tenant_id,
                actor_type: "USER".to_string(),
                actor_user_id: Some(ctx.user_id),
                action: OP_ACTION_OMS_ORDER_MERGE_SOURCE.to_string(),
                ref_type: Some("ORDER".to_string()),
                ref_id: Some(detail.order.id.to_string()),
                metadata: Some(serde_json::json!({
                    "target_order_id": target_order_id.to_string(),
                    "force": force_merge,
                })),
            })
            .await;
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_MERGE_TARGET",
        "ORDER",
        Some(target_order_id.to_string()),
        None,
        Some(serde_json::json!({
            "merged_order_ids": merged_order_ids.clone(),
            "force": force_merge,
        })),
    )
    .await;

    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_OMS_ORDER_MERGE_TARGET.to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(target_order_id.to_string()),
            metadata: Some(serde_json::json!({
                "merged_order_ids": merged_order_ids.clone(),
                "force": force_merge,
            })),
        })
        .await;

    let response = OrderMergeResponse {
        target_order_id: target_order_id.to_string(),
        merged_order_ids,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn split_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderSplitRequest>,
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
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
        return resp.into_response();
    }
    if !detail.order.review_status.is_pending() {
        return bad_request("order not pending review").into_response();
    }
    if matches!(
        detail.order.oms_status,
        OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
    ) {
        return bad_request("order already shipped").into_response();
    }
    if detail
        .order
        .exception_flags
        .iter()
        .any(|flag| flag == ORDER_SPLIT_FLAG)
    {
        return bad_request("order already split").into_response();
    }
    if detail.items.is_empty() {
        return bad_request("order has no items").into_response();
    }

    let plan = if let Some(children) = req.children {
        if children.len() < 2 {
            return bad_request("split children required").into_response();
        }
        let mut parsed = Vec::with_capacity(children.len());
        for child in children {
            if child.items.is_empty() {
                return bad_request("split child empty").into_response();
            }
            let items = child
                .items
                .into_iter()
                .map(|item| OrderSplitPlanItem {
                    line_id: item.line_id.trim().to_string(),
                    qty: item.qty,
                })
                .collect::<Vec<_>>();
            parsed.push(OrderSplitPlanChild { items });
        }
        parsed
    } else {
        let Some(split_count) = req.split_count else {
            return bad_request("split_count required").into_response();
        };
        if split_count < 2 {
            return bad_request("split_count invalid").into_response();
        }
        let split_count = split_count as usize;
        let mut children = vec![OrderSplitPlanChild { items: Vec::new() }; split_count];
        for item in &detail.items {
            let mut remaining = item.quantity;
            for (idx, child) in children.iter_mut().enumerate() {
                if remaining <= 0 {
                    break;
                }
                let slots_left = (split_count - idx) as i32;
                let qty = if idx == split_count - 1 {
                    remaining
                } else {
                    (remaining + slots_left - 1) / slots_left
                };
                if qty > 0 {
                    child.items.push(OrderSplitPlanItem {
                        line_id: item.id.clone(),
                        qty,
                    });
                    remaining -= qty;
                }
            }
        }
        children
    };

    let mut item_qty_map = std::collections::HashMap::new();
    for item in &detail.items {
        item_qty_map.insert(item.id.clone(), item.quantity);
    }
    let mut plan_qty_map: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for child in &plan {
        if child.items.is_empty() {
            return bad_request("split child empty").into_response();
        }
        for item in &child.items {
            if item.line_id.trim().is_empty() {
                return bad_request("split item line_id required").into_response();
            }
            if item.qty <= 0 {
                return bad_request("split item qty invalid").into_response();
            }
            if !item_qty_map.contains_key(&item.line_id) {
                return bad_request("split item not found").into_response();
            }
            *plan_qty_map.entry(item.line_id.clone()).or_insert(0) += item.qty;
        }
    }
    for (line_id, qty) in &item_qty_map {
        if plan_qty_map.get(line_id) != Some(qty) {
            return bad_request("split qty mismatch").into_response();
        }
    }
    if plan_qty_map.len() != item_qty_map.len() {
        return bad_request("split plan missing items").into_response();
    }

    let split_count = plan.len() as i32;
    let split_result = match orders_repo.split_order(tenant_id, order_id, plan).await {
        Ok(v) => v,
        Err(err) => {
            let message = err.to_string();
            if message.contains("order") || message.contains("split") {
                return bad_request(&message).into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
    };

    let child_order_ids = split_result
        .child_orders
        .iter()
        .map(|child| child.order_id.to_string())
        .collect::<Vec<_>>();

    let mut next_flags = detail.order.exception_flags.clone();
    if !next_flags.iter().any(|flag| flag == ORDER_SPLIT_FLAG) {
        next_flags.push(ORDER_SPLIT_FLAG.to_string());
    }

    record_order_state_logs(
        &state,
        &req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: "OMS.ORDER_SPLIT_SOURCE".to_string(),
            from_status: detail.order.order_status,
            to_status: OrderMainStatus::Cancelled,
            operator_id: Some(ctx.user_id),
            reason: None,
            request_payload: serde_json::json!({
                "split_count": split_count,
                "child_order_ids": child_order_ids.clone(),
            }),
            response_payload: serde_json::json!({
                "oms_status": OmsOrderStatus::Cancelled.to_string(),
                "order_status": OrderMainStatus::Cancelled.to_string(),
                "review_status": OrderReviewStatus::Rejected.to_string(),
                "exception_flags": next_flags.clone(),
            }),
        },
    )
    .await;

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_SPLIT_SOURCE",
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!({
            "oms_status": detail.order.oms_status.to_string(),
            "review_status": detail.order.review_status.to_string(),
            "exception_flags": detail.order.exception_flags,
        })),
        Some(serde_json::json!({
            "oms_status": OmsOrderStatus::Cancelled.to_string(),
            "review_status": OrderReviewStatus::Rejected.to_string(),
            "exception_flags": next_flags,
            "child_order_ids": child_order_ids.clone(),
        })),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_OMS_ORDER_SPLIT_SOURCE.to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "child_order_ids": child_order_ids.clone(),
                "split_count": split_count,
            })),
        })
        .await;

    for child in &split_result.child_orders {
        record_audit_log(
            &state,
            &ctx,
            &req_ctx,
            "OMS.ORDER_SPLIT_CHILD",
            "ORDER",
            Some(child.order_id.to_string()),
            None,
            Some(serde_json::json!({
                "source_order_id": order_id.to_string(),
                "items": child.items.iter().map(|item| serde_json::json!({
                    "line_id": item.line_id,
                    "sku_id": item.sku_id,
                    "title": item.title,
                    "qty": item.qty,
                })).collect::<Vec<_>>(),
            })),
        )
        .await;

        let _ = logs_repo
            .insert(OperationLogCreate {
                tenant_id,
                actor_type: "USER".to_string(),
                actor_user_id: Some(ctx.user_id),
                action: OP_ACTION_OMS_ORDER_SPLIT_CHILD.to_string(),
                ref_type: Some("ORDER".to_string()),
                ref_id: Some(child.order_id.to_string()),
                metadata: Some(serde_json::json!({
                    "source_order_id": order_id.to_string(),
                })),
            })
            .await;
    }

    let response = OrderSplitResponse {
        source_order_id: order_id.to_string(),
        split_count,
        created_at: Utc::now(),
        child_orders: split_result
            .child_orders
            .into_iter()
            .map(|child| OrderSplitChildResponse {
                order_id: child.order_id.to_string(),
                items: child
                    .items
                    .into_iter()
                    .map(|item| OrderSplitItemResponse {
                        line_id: item.line_id,
                        sku_id: item.sku_id,
                        title: item.title,
                        qty: item.qty,
                        price_cent: item.price_cent,
                    })
                    .collect(),
            })
            .collect(),
    };

    (StatusCode::OK, Json(response)).into_response()
}
