use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;
use tokio::sync::Mutex;

pub(super) struct NormalizedOrderDownloadRequest {
    pub(super) cursor: Option<String>,
    pub(super) watermark: Option<DateTime<Utc>>,
}

pub(super) struct NormalizedManualOrderItems {
    pub(super) items: Vec<OrderItemProjection>,
    pub(super) computed_total_cent: i64,
}

const MANUAL_ORDER_CREATE_LOCK_SHARDS: usize = 64;

fn manual_order_create_locks() -> &'static Vec<Mutex<()>> {
    static LOCKS: OnceLock<Vec<Mutex<()>>> = OnceLock::new();
    LOCKS.get_or_init(|| {
        (0..MANUAL_ORDER_CREATE_LOCK_SHARDS)
            .map(|_| Mutex::new(()))
            .collect()
    })
}

fn manual_order_create_lock_index(
    tenant_id: TenantId,
    platform: Platform,
    platform_order_id: &str,
) -> usize {
    let mut hasher = DefaultHasher::new();
    tenant_id.hash(&mut hasher);
    platform.hash(&mut hasher);
    platform_order_id.hash(&mut hasher);
    (hasher.finish() as usize) % MANUAL_ORDER_CREATE_LOCK_SHARDS
}

fn normalize_optional_text(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_optional_rfc3339_allow_empty(
    raw: Option<&str>,
    field_name: &str,
) -> Result<Option<DateTime<Utc>>, Response> {
    let Some(value) = normalize_optional_text(raw) else {
        return Ok(None);
    };
    match DateTime::parse_from_rfc3339(&value) {
        Ok(parsed) => Ok(Some(parsed.with_timezone(&Utc))),
        Err(_) => {
            let message = format!("bad {field_name}");
            Err(bad_request(&message).into_response())
        }
    }
}

fn parse_optional_rfc3339_strict(
    raw: Option<&str>,
    field_name: &str,
) -> Result<Option<DateTime<Utc>>, Response> {
    let Some(value) = raw else {
        return Ok(None);
    };
    match DateTime::parse_from_rfc3339(value.trim()) {
        Ok(parsed) => Ok(Some(parsed.with_timezone(&Utc))),
        Err(_) => {
            let message = format!("bad {field_name}");
            Err(bad_request(&message).into_response())
        }
    }
}

pub(super) fn normalize_download_request(
    req: &OrderDownloadRequest,
) -> Result<NormalizedOrderDownloadRequest, Response> {
    Ok(NormalizedOrderDownloadRequest {
        cursor: normalize_optional_text(req.cursor.as_deref()),
        watermark: parse_optional_rfc3339_allow_empty(req.watermark.as_deref(), "watermark")?,
    })
}

pub(super) fn normalize_manual_order_items(
    items: &[ManualOrderItemRequest],
) -> Result<NormalizedManualOrderItems, Response> {
    let mut projections = Vec::with_capacity(items.len());
    let mut line_ids = HashSet::new();
    let mut computed_total_cent = 0_i64;

    for (idx, item) in items.iter().enumerate() {
        if item.qty <= 0 {
            return Err(bad_request("item qty must be positive").into_response());
        }
        if item.price_cent < 0 {
            return Err(bad_request("item price_cent must be non-negative").into_response());
        }

        let line_id = normalize_optional_text(item.line_id.as_deref())
            .unwrap_or_else(|| format!("LINE-{}", idx + 1));
        if !line_ids.insert(line_id.clone()) {
            return Err(bad_request("duplicated line_id").into_response());
        }

        let amount_cent = match item.amount_cent {
            Some(value) => value,
            None => match item.price_cent.checked_mul(i64::from(item.qty)) {
                Some(value) => value,
                None => return Err(bad_request("item amount overflow").into_response()),
            },
        };
        if amount_cent < 0 {
            return Err(bad_request("item amount_cent must be non-negative").into_response());
        }

        if let Some(pay_amount_cent) = item.pay_amount_cent {
            if pay_amount_cent < 0 {
                return Err(
                    bad_request("item pay_amount_cent must be non-negative").into_response()
                );
            }
            if pay_amount_cent > amount_cent {
                return Err(
                    bad_request("item pay_amount_cent must be <= amount_cent").into_response()
                );
            }
        }
        if let Some(discount_cent) = item.discount_cent {
            if discount_cent < 0 {
                return Err(bad_request("item discount_cent must be non-negative").into_response());
            }
            if discount_cent > amount_cent {
                return Err(bad_request("item discount_cent must be <= amount_cent").into_response());
            }
        }

        computed_total_cent = match computed_total_cent.checked_add(amount_cent) {
            Some(value) => value,
            None => return Err(bad_request("items total overflow").into_response()),
        };

        projections.push(OrderItemProjection {
            line_id,
            sku_id: normalize_optional_text(item.sku_id.as_deref()),
            title: normalize_optional_text(item.title.as_deref()),
            qty: item.qty,
            price_cent: item.price_cent,
            amount_cent,
            pay_amount_cent: item.pay_amount_cent.or(Some(amount_cent)),
            discount_cent: item.discount_cent,
            refund_state: RefundState::None,
            after_sale_state: AfterSaleState::None,
            refund_amount_cent: None,
            after_sale_type: None,
            after_sale_id: None,
            raw: serde_json::json!({}),
        });
    }

    Ok(NormalizedManualOrderItems {
        items: projections,
        computed_total_cent,
    })
}

pub(super) async fn download_orders(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.download") {
        return resp.into_response();
    }
    let shop_id = match ShopId::from_str(&req.shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }

    let sync_enabled =
        match feature_flag_enabled(&state, tenant_id, Some(shop_id), FeatureFlagKey::OrderSync)
            .await
        {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !sync_enabled {
        return feature_flag_forbidden(FeatureFlagKey::OrderSync).into_response();
    }

    let normalized = match normalize_download_request(&req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let sync_repo = SyncJobsRepo::new(state.db.pool());
    let job = match sync_repo
        .upsert_job(SyncJobUpsert {
            tenant_id,
            shop_id,
            job_type: "order_sync".to_string(),
            cursor: normalized.cursor,
            watermark: normalized.watermark,
            status: Some("PENDING".to_string()),
            last_error: Some("MANUAL_DOWNLOAD".to_string()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let runs_repo = SyncJobRunsRepo::new(state.db.pool());
    let now = Utc::now();
    if let Err(e) = runs_repo
        .insert_run(SyncJobRunCreate {
            tenant_id,
            job_id: job.job_id,
            shop_id: job.shop_id,
            job_type: job.job_type.clone(),
            status: "REQUESTED".to_string(),
            item_count: 0,
            duration_ms: 0,
            error: String::new(),
            started_at: now,
            finished_at: now,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER.DOWNLOAD.REQUESTED",
        "sync_job",
        Some(job.job_id.to_string()),
        None,
        Some(serde_json::json!({
            "shop_id": job.shop_id,
            "job_type": job.job_type,
        })),
    )
    .await;

    (StatusCode::ACCEPTED, Json(job)).into_response()
}

pub(super) async fn create_manual_order(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ManualOrderCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.create") {
        return resp.into_response();
    }

    let shop_id = match ShopId::from_str(&req.shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }

    let platform_order_id = req.platform_order_id.trim();
    if platform_order_id.is_empty() {
        return bad_request("platform_order_id required").into_response();
    }
    if req.items.is_empty() {
        return bad_request("items required").into_response();
    }

    let shops_repo = ShopsRepo::new(state.db.pool());
    let shop = match shops_repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let order_created_at =
        match parse_optional_rfc3339_strict(req.order_created_at.as_deref(), "order_created_at") {
            Ok(Some(v)) => v,
            Ok(None) => Utc::now(),
            Err(resp) => return resp,
        };
    let platform_updated_at = match parse_optional_rfc3339_strict(
        req.platform_updated_at.as_deref(),
        "platform_updated_at",
    ) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let normalized_items = match normalize_manual_order_items(&req.items) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let total_amount_cent = req
        .total_amount_cent
        .unwrap_or(normalized_items.computed_total_cent);
    if total_amount_cent < 0 {
        return bad_request("total_amount_cent must be non-negative").into_response();
    }
    let goods_amount_cent = req
        .goods_amount_cent
        .or(Some(normalized_items.computed_total_cent));
    if let Some(value) = goods_amount_cent {
        if value < 0 {
            return bad_request("goods_amount_cent must be non-negative").into_response();
        }
    }
    if let Some(value) = req.postage_amount_cent {
        if value < 0 {
            return bad_request("postage_amount_cent must be non-negative").into_response();
        }
    }
    if let Some(value) = req.discount_amount_cent {
        if value < 0 {
            return bad_request("discount_amount_cent must be non-negative").into_response();
        }
    }
    let currency =
        normalize_optional_text(req.currency.as_deref()).unwrap_or_else(|| "CNY".to_string());
    if currency.len() > 16 {
        return bad_request("currency too long").into_response();
    }

    let projection = OrderProjection {
        tenant_id,
        shop_id,
        platform: shop.platform,
        platform_order_id: platform_order_id.to_string(),
        status: OrderStatus::PendingShipment,
        total_amount_cent,
        goods_amount_cent,
        postage_amount_cent: req.postage_amount_cent,
        discount_amount_cent: req.discount_amount_cent,
        platform_discount_cent: None,
        seller_discount_cent: None,
        coupon_discount_cent: None,
        currency,
        buyer_name: normalize_optional_text(req.buyer_name.as_deref()),
        receiver_name: normalize_optional_text(req.receiver_name.as_deref()),
        receiver_phone: normalize_optional_text(req.receiver_phone.as_deref()),
        receiver_address: normalize_optional_text(req.receiver_address.as_deref()),
        pay_at: None,
        ship_at: None,
        finish_at: None,
        cancel_at: None,
        buyer_message: normalize_optional_text(req.buyer_message.as_deref()),
        seller_remark: normalize_optional_text(req.seller_remark.as_deref()),
        logistics_company: normalize_optional_text(req.logistics_company.as_deref()),
        tracking_no: normalize_optional_text(req.tracking_no.as_deref()),
        order_created_at,
        platform_updated_at,
        raw: serde_json::to_value(&req).unwrap_or_else(|_| serde_json::json!({})),
        items: normalized_items.items,
    };

    // Keep lock scope minimal: guard only dedup check and upsert to reduce contention.
    let order_id = {
        let lock_idx = manual_order_create_lock_index(tenant_id, shop.platform, platform_order_id);
        let _create_guard = manual_order_create_locks()[lock_idx].lock().await;

        let orders_repo = OrdersRepo::new(state.db.pool());
        match orders_repo
            .locate_by_platform_order_id(tenant_id, shop.platform, platform_order_id)
            .await
        {
            Ok(Some(_)) => return conflict("platform_order_id already exists").into_response(),
            Ok(None) => {}
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }

        match orders_repo.upsert_order_projection(&projection).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER.CREATE",
        "ORDER",
        Some(order_id.to_string()),
        None,
        Some(serde_json::json!({
            "shop_id": shop_id,
            "platform": shop.platform.to_string(),
            "platform_order_id": platform_order_id,
            "item_count": detail.items.len(),
            "total_amount_cent": detail.order.total_amount_cent,
        })),
    )
    .await;

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
    (StatusCode::CREATED, Json(response)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item() -> ManualOrderItemRequest {
        ManualOrderItemRequest {
            line_id: Some("LINE-1".to_string()),
            sku_id: Some("SKU-1".to_string()),
            title: Some("sample".to_string()),
            qty: 1,
            price_cent: 100,
            amount_cent: Some(100),
            pay_amount_cent: Some(100),
            discount_cent: Some(0),
        }
    }

    #[test]
    fn intake_rejects_negative_item_amount_cent() {
        let mut item = sample_item();
        item.amount_cent = Some(-1);

        let result = normalize_manual_order_items(&[item]);
        assert!(result.is_err());
    }

    #[test]
    fn intake_rejects_negative_item_pay_amount_cent() {
        let mut item = sample_item();
        item.pay_amount_cent = Some(-1);

        let result = normalize_manual_order_items(&[item]);
        assert!(result.is_err());
    }

    #[test]
    fn intake_rejects_negative_item_discount_cent() {
        let mut item = sample_item();
        item.discount_cent = Some(-1);

        let result = normalize_manual_order_items(&[item]);
        assert!(result.is_err());
    }

    #[test]
    fn intake_rejects_overflow_when_computing_item_amount() {
        let mut item = sample_item();
        item.qty = i32::MAX;
        item.price_cent = i64::MAX;
        item.amount_cent = None;
        item.pay_amount_cent = None;
        item.discount_cent = None;

        let result = normalize_manual_order_items(&[item]);
        assert!(result.is_err());
    }

    #[test]
    fn intake_rejects_item_pay_amount_greater_than_amount() {
        let mut item = sample_item();
        item.amount_cent = Some(100);
        item.pay_amount_cent = Some(101);

        let result = normalize_manual_order_items(&[item]);
        assert!(result.is_err());
    }

    #[test]
    fn intake_rejects_item_discount_greater_than_amount() {
        let mut item = sample_item();
        item.amount_cent = Some(100);
        item.discount_cent = Some(101);

        let result = normalize_manual_order_items(&[item]);
        assert!(result.is_err());
    }

    #[test]
    fn intake_rejects_items_total_overflow() {
        let mut item_a = sample_item();
        item_a.amount_cent = Some(i64::MAX);
        item_a.pay_amount_cent = Some(i64::MAX);
        item_a.discount_cent = Some(0);

        let mut item_b = sample_item();
        item_b.line_id = Some("LINE-2".to_string());
        item_b.amount_cent = Some(1);
        item_b.pay_amount_cent = Some(1);
        item_b.discount_cent = Some(0);

        let result = normalize_manual_order_items(&[item_a, item_b]);
        assert!(result.is_err());
    }

    #[test]
    fn intake_manual_order_lock_shard_is_stable_for_same_key() {
        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let platform = Platform::Taobao;
        let key = "PLAT-ORDER-001";

        let first = manual_order_create_lock_index(tenant_id, platform, key);
        let second = manual_order_create_lock_index(tenant_id, platform, key);

        assert_eq!(first, second);
        assert!(first < MANUAL_ORDER_CREATE_LOCK_SHARDS);
    }
}
