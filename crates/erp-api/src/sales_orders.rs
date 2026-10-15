use super::*;
mod quantity;
mod revision_lines;
mod documents;
use quantity::SalesQuantity;
use erp_repo::sales_orders::{
    SalesOrder, SalesOrderCreate, SalesOrderDetail, SalesOrderItem, SalesOrderItemInput,
    SalesOrderListFilters, SalesOrderStatusUpdate, SalesOrderUpdate, SalesOrdersRepo,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/tenants/:tenant_id/trade-sales-order-lookup",get(documents::lookup))
        .route("/v1/tenants/:tenant_id/trade-sales-order-commands/:request_id",get(documents::command))
        .route("/v1/tenants/:tenant_id/trade-sales-orders",get(documents::list).post(documents::save))
        .route("/v1/tenants/:tenant_id/trade-sales-orders/:so_id",get(documents::read))
        .route(
            "/v1/tenants/:tenant_id/sales-orders",
            get(list_sales_orders).post(create_sales_order),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-orders/:sales_order_id",
            get(get_sales_order).put(update_sales_order),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-orders/:sales_order_id/status",
            patch(update_sales_order_status),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-orders/:sales_order_id/confirm",
            post(confirm_scaled_trade_sale_route),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-orders/:sales_order_id/deliveries",
            post(create_sales_delivery),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-orders/:sales_order_id/returns",
            post(create_sales_return),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-orders/:sales_order_id/returns/:return_id",
            get(read_sales_return),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-orders/:sales_order_id/actions",
            get(list_sales_actions).post(create_sales_action),
        )
}

#[derive(Debug, Deserialize)]
struct SalesOrderListQuery {
    shop_id: Option<String>,
    sales_order_status: Option<String>,
    delivery_status: Option<String>,
    payment_status: Option<String>,
    refund_status: Option<String>,
    customer_id: Option<String>,
    sales_order_no: Option<String>,
    keyword: Option<String>,
    source_order_id: Option<String>,
    wms_outbound_id: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    updated_from: Option<String>,
    updated_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SalesOrderItemRequest {
    line_id: Option<String>,
    sku_id: Option<String>,
    sku_code: Option<String>,
    sku_name: Option<String>,
    qty: SalesQuantity,
    line_no: Option<i32>,
    tax_cent: Option<i64>,
    due_date: Option<String>,
    address: Option<String>,
    source_kind: Option<String>,
    price_cent: i64,
    tax_rate: Option<f64>,
    discount_cent: Option<i64>,
    amount_cent: Option<i64>,
    warehouse_id: Option<String>,
    delivery_deadline: Option<String>,
    extra: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct SalesOrderCreateRequest {
    shop_id: Option<String>,
    sales_order_no: String,
    source_type: String,
    source_order_ids: Option<Vec<String>>,
    customer_id: Option<String>,
    currency_code: Option<String>,
    tax_mode: Option<String>,
    sales_order_status: Option<String>,
    delivery_status: Option<String>,
    invoice_status: Option<String>,
    payment_status: Option<String>,
    refund_status: Option<String>,
    return_status: Option<String>,
    stock_status: Option<String>,
    total_amount_cent: Option<i64>,
    discount_amount_cent: Option<i64>,
    shipping_amount_cent: Option<i64>,
    receivable_amount_cent: Option<i64>,
    paid_amount_cent: Option<i64>,
    refunded_amount_cent: Option<i64>,
    invoice_amount_cent: Option<i64>,
    status_flags: Option<Vec<String>>,
    remark: Option<String>,
    receiver_name: Option<String>,
    receiver_phone: Option<String>,
    receiver_address: Option<String>,
    receiver_province: Option<String>,
    receiver_city: Option<String>,
    receiver_district: Option<String>,
    receiver_postcode: Option<String>,
    carrier_code: Option<String>,
    logistics_service: Option<String>,
    expected_ship_at: Option<String>,
    review_status: Option<String>,
    fulfillment_status: Option<String>,
    exception_status: Option<String>,
    freeze_reason: Option<String>,
    exception_flags: Option<Vec<String>>,
    items: Vec<SalesOrderItemRequest>,
    /// Shared command identity. Present only for the decimal trade sale path.
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    confirm_balance: Option<bool>,
    #[serde(default)]
    partner_code: Option<String>,
    #[serde(default)]
    revision_action: Option<String>,
    #[serde(default)]
    so_id: Option<String>,
    #[serde(default)]
    expected_version: Option<i32>,
    #[serde(default)]
    customer_code: Option<String>,
    #[serde(default)]
    due_date: Option<String>,
    #[serde(default)]
    address: Option<String>,
    #[serde(default)]
    source_kind: Option<String>,
    #[serde(default)]
    tax_cent: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SalesOrderUpdateRequest {
    shop_id: Option<String>,
    sales_order_no: Option<String>,
    source_type: Option<String>,
    source_order_ids: Option<Vec<String>>,
    customer_id: Option<String>,
    currency_code: Option<String>,
    tax_mode: Option<String>,
    sales_order_status: Option<String>,
    delivery_status: Option<String>,
    invoice_status: Option<String>,
    payment_status: Option<String>,
    refund_status: Option<String>,
    return_status: Option<String>,
    stock_status: Option<String>,
    total_amount_cent: Option<i64>,
    discount_amount_cent: Option<i64>,
    shipping_amount_cent: Option<i64>,
    receivable_amount_cent: Option<i64>,
    paid_amount_cent: Option<i64>,
    refunded_amount_cent: Option<i64>,
    invoice_amount_cent: Option<i64>,
    status_flags: Option<Vec<String>>,
    remark: Option<String>,
    receiver_name: Option<String>,
    receiver_phone: Option<String>,
    receiver_address: Option<String>,
    receiver_province: Option<String>,
    receiver_city: Option<String>,
    receiver_district: Option<String>,
    receiver_postcode: Option<String>,
    carrier_code: Option<String>,
    logistics_service: Option<String>,
    expected_ship_at: Option<String>,
    review_status: Option<String>,
    fulfillment_status: Option<String>,
    exception_status: Option<String>,
    freeze_reason: Option<String>,
    exception_flags: Option<Vec<String>>,
    items: Option<Vec<SalesOrderItemRequest>>,
}

#[derive(Debug, Deserialize)]
struct SalesOrderStatusRequest {
    sales_order_status: Option<String>,
    delivery_status: Option<String>,
    invoice_status: Option<String>,
    payment_status: Option<String>,
    refund_status: Option<String>,
    return_status: Option<String>,
    stock_status: Option<String>,
    status_flags: Option<Vec<String>>,
    review_status: Option<String>,
    fulfillment_status: Option<String>,
    exception_status: Option<String>,
    freeze_reason: Option<String>,
    exception_flags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct SalesOrderListResponse {
    items: Vec<SalesOrderResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct SalesOrderResponse {
    sales_order_id: uuid::Uuid,
    shop_id: Option<uuid::Uuid>,
    sales_order_no: String,
    source_type: String,
    source_order_ids: Vec<String>,
    customer_id: Option<uuid::Uuid>,
    currency_code: String,
    tax_mode: Option<String>,
    sales_order_status: String,
    delivery_status: String,
    invoice_status: String,
    payment_status: String,
    refund_status: String,
    return_status: Option<String>,
    stock_status: Option<String>,
    total_amount_cent: i64,
    discount_amount_cent: i64,
    shipping_amount_cent: i64,
    receivable_amount_cent: i64,
    paid_amount_cent: i64,
    refunded_amount_cent: i64,
    invoice_amount_cent: i64,
    status_flags: Vec<String>,
    remark: Option<String>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    created_by: Option<uuid::Uuid>,
    updated_by: Option<uuid::Uuid>,
    receiver_name: Option<String>,
    receiver_phone: Option<String>,
    receiver_address: Option<String>,
    receiver_province: Option<String>,
    receiver_city: Option<String>,
    receiver_district: Option<String>,
    receiver_postcode: Option<String>,
    carrier_code: Option<String>,
    logistics_service: Option<String>,
    expected_ship_at: Option<DateTime<Utc>>,
    wms_outbound_id: Option<uuid::Uuid>,
    wms_outbound_status: Option<String>,
    wms_wave_id: Option<uuid::Uuid>,
    wms_wave_status: Option<String>,
    wms_ship_status: Option<String>,
    package_count: i32,
    waybill_nos: Vec<String>,
    review_status: Option<String>,
    fulfillment_status: Option<String>,
    exception_status: Option<String>,
    freeze_reason: Option<String>,
    exception_flags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SalesOrderItemResponse {
    line_id: uuid::Uuid,
    sku_id: Option<uuid::Uuid>,
    sku_code: Option<String>,
    sku_name: Option<String>,
    qty: i32,
    price_cent: i64,
    tax_rate: Option<f64>,
    discount_cent: i64,
    amount_cent: i64,
    warehouse_id: Option<uuid::Uuid>,
    delivery_deadline: Option<DateTime<Utc>>,
    extra: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct SalesOrderDetailResponse {
    order: SalesOrderResponse,
    items: Vec<SalesOrderItemResponse>,
}

#[derive(Debug)]
struct ValidationError {
    message: String,
}

impl ValidationError {
    fn invalid(label: &str) -> Self {
        Self {
            message: format!("{label} 无效"),
        }
    }

    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }

    fn into_response(self) -> Response {
        bad_request(&self.message).into_response()
    }
}

async fn list_sales_orders(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SalesOrderListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let parse_uuid = |raw: Option<String>, label: &str| -> Result<Option<uuid::Uuid>, Response> {
        let Some(raw) = raw.and_then(non_empty) else {
            return Ok(None);
        };
        let id = uuid::Uuid::parse_str(&raw)
            .map_err(|_| bad_request(&format!("{label} 无效")).into_response())?;
        Ok(Some(id))
    };

    let shop_id = match parse_uuid(query.shop_id, "shop_id") {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let customer_id = match parse_uuid(query.customer_id, "customer_id") {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let wms_outbound_id = match parse_uuid(query.wms_outbound_id, "wms_outbound_id") {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let parse_dt = |raw: Option<String>, label: &str| -> Result<Option<DateTime<Utc>>, Response> {
        let Some(raw) = raw.and_then(non_empty) else {
            return Ok(None);
        };
        let dt = DateTime::parse_from_rfc3339(&raw)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|_| bad_request(&format!("{label} 无效")).into_response())?;
        Ok(Some(dt))
    };

    let created_from = match parse_dt(query.created_from, "created_from") {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let created_to = match parse_dt(query.created_to, "created_to") {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let updated_from = match parse_dt(query.updated_from, "updated_from") {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let updated_to = match parse_dt(query.updated_to, "updated_to") {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let filters = SalesOrderListFilters {
        shop_id,
        sales_order_status: query.sales_order_status.and_then(non_empty),
        delivery_status: query.delivery_status.and_then(non_empty),
        payment_status: query.payment_status.and_then(non_empty),
        refund_status: query.refund_status.and_then(non_empty),
        customer_id,
        sales_order_no: query.sales_order_no.and_then(non_empty),
        keyword: query.keyword.and_then(non_empty),
        source_order_id: query.source_order_id.and_then(non_empty),
        wms_outbound_id,
        created_from,
        created_to,
        updated_from,
        updated_to,
        offset,
        limit,
    };

    let repo = SalesOrdersRepo::new(state.db.pool());
    let result = match repo.list_sales_orders(tenant_id, filters).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result.items.into_iter().map(sales_order_response).collect();
    (
        StatusCode::OK,
        Json(SalesOrderListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

async fn get_sales_order(
    State(state): State<AppState>,
    Path((tenant_id, sales_order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let sales_order_id = match uuid::Uuid::parse_str(&sales_order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("sales_order_id 无效").into_response(),
    };

    let repo = SalesOrdersRepo::new(state.db.pool());
    let detail = match repo.get_sales_order_detail(tenant_id, sales_order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("销售单不存在").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = sales_order_detail_response(detail);
    (StatusCode::OK, Json(response)).into_response()
}

#[derive(Debug, Deserialize)]
struct SalesDeliveryRequest {
    request_id: String,
    line_no: i32,
    expected_version: i32,
    qty_micros: i64,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    delivery_id: Option<String>,
    #[serde(default)]
    carrier_ref: Option<String>,
    #[serde(default)]
    warehouse_id: Option<String>,
    #[serde(default)]
    location_id: Option<String>,
    #[serde(default)]
    sku_id: Option<String>,
    #[serde(default)]
    batch_code: Option<String>,
    #[serde(default)]
    delivery_request: Option<String>,
    #[serde(default)]
    cancel_qty_micros: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SalesReturnRequest {
    request_id: String,
    delivery_id: String,
    good_micros: i64,
    rejected_micros: i64,
    unit_cost_cent: i64,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    customer_code: String,
    #[serde(default)]
    cash_offset_cent: Option<i64>,
    #[serde(default)]
    balance_id: Option<String>,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    account_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SalesActionRequest {
    request_id: String,
    action: String,
    actor_user_id: String,
    #[serde(default)]
    has_downstream: bool,
    #[serde(default)]
    expected_version: Option<i32>,
}

async fn create_sales_action(
    State(state): State<AppState>,
    Path((tenant_id, sales_order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SalesActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    let document_id = match uuid::Uuid::parse_str(&sales_order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("sales_order_id 无效").into_response(),
    };
    let saved = erp_repo::shared_contract::SharedContractRepo::new(state.db.pool())
        .record_document_action(
            tenant_id,
            &req.request_id,
            &req.request_id,
            "SALES",
            document_id,
            &req.action,
            &req.actor_user_id,
            req.has_downstream,
            req.expected_version,
        )
        .await;
    let (write, action_id, status) = match saved {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => return conflict("payload conflict").into_response(),
        Err(error) => return (StatusCode::CONFLICT, format!("{error}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "action_id": action_id,
            "status": status,
            "replay": matches!(write, erp_repo::shared_contract::SharedWrite::Replay(_)),
        })),
    )
        .into_response()
}

async fn list_sales_actions(
    State(state): State<AppState>,
    Path((tenant_id, sales_order_id)): Path<(String, String)>,
    Query(query): Query<SalesOrderListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    let document_id = match uuid::Uuid::parse_str(&sales_order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("sales_order_id 无效").into_response(),
    };
    let page = erp_repo::shared_contract::SharedContractRepo::new(state.db.pool())
        .read_document_actions(tenant_id, "SALES", document_id, query.offset.unwrap_or(0), query.limit.unwrap_or(20))
        .await;
    let (total, rows) = match page {
        Ok(value) => value,
        Err(error) => return (StatusCode::BAD_REQUEST, format!("{error}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "total": total,
            "items": rows.into_iter().map(|(id, action, status, version, reverses)| serde_json::json!({
                "action_id": id,
                "action": action,
                "status": status,
                "version_no": version,
                "reverses_action_id": reverses,
            })).collect::<Vec<_>>(),
        })),
    )
        .into_response()
}

async fn read_sales_return(
    State(state): State<AppState>,
    Path((tenant_id, _sales_order_id, return_id)): Path<(String, String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    let return_id = match uuid::Uuid::parse_str(&return_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad return_id").into_response(),
    };
    match erp_repo::shared_contract::SharedContractRepo::new(state.db.pool())
        .read_sales_return(tenant_id, return_id)
        .await
    {
        Ok(Some((delivery_id, good, rejected, cost, credit, posted))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "return_id": return_id,
                "delivery_id": delivery_id,
                "good_micros": good,
                "rejected_micros": rejected,
                "cost_cent": cost,
                "credit_cent": credit,
                "posted_cent": posted,
            })),
        )
            .into_response(),
        Ok(None) => not_found("退货不存在").into_response(),
        Err(error) => (StatusCode::CONFLICT, format!("{error}")).into_response(),
    }
}

async fn create_sales_return(
    State(state): State<AppState>,
    Path((tenant_id, _sales_order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SalesReturnRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    let delivery_id = match uuid::Uuid::parse_str(&req.delivery_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad delivery_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(&req.location_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    if let Some(offset) = req.cash_offset_cent.filter(|value| *value > 0) {
        let balance_id = match req.balance_id.as_deref().and_then(|value| uuid::Uuid::parse_str(value).ok()) {
            Some(value) => value,
            None => return bad_request("bad balance_id").into_response(),
        };
        let saved = erp_repo::shared_contract::SharedContractRepo::new(state.db.pool())
            .save_return_cash_offset(erp_repo::shared_contract::ReturnCashOffsetInput { tenant_id, request_id: &req.request_id, payload_hash: &req.request_id, balance_id, partner_code: &req.customer_code, currency: req.currency.as_deref().unwrap_or(""), account_code: req.account_code.as_deref().unwrap_or(""), return_cent: offset })
            .await;
        let (write, left) = match saved {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => return conflict("payload conflict").into_response(),
            Err(error) => return (StatusCode::CONFLICT, format!("{error}")).into_response(),
        };
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "open_cent": left,
                "replay": matches!(write, erp_repo::shared_contract::SharedWrite::Replay(_)),
            })),
        )
            .into_response();
    }
    let saved = erp_repo::shared_contract::SharedContractRepo::new(state.db.pool())
        .return_sales_delivery(
            tenant_id,
            &req.request_id,
            &req.request_id,
            delivery_id,
            req.good_micros,
            req.rejected_micros,
            req.unit_cost_cent,
            warehouse_id,
            location_id,
            sku_id,
            &req.batch_code,
            &req.customer_code,
        )
        .await;
    let (write, return_id, credit) = match saved {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => return conflict("payload conflict").into_response(),
        Err(error) => return (StatusCode::CONFLICT, format!("{error}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "return_id": return_id,
            "credit_cent": credit,
            "shop_required": false,
            "replay": matches!(write, erp_repo::shared_contract::SharedWrite::Replay(_)),
        })),
    )
        .into_response()
}

async fn create_sales_delivery(
    State(state): State<AppState>,
    Path((tenant_id, sales_order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SalesDeliveryRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    let so_id = match uuid::Uuid::parse_str(&sales_order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("sales_order_id 无效").into_response(),
    };
    let repo = erp_repo::shared_contract::SharedContractRepo::new(state.db.pool());
    if let Some(cancel_qty) = req.cancel_qty_micros.filter(|qty| *qty > 0) {
        let saved = repo
            .cancel_sales_remainder(
                tenant_id,
                &req.request_id,
                &req.request_id,
                so_id,
                req.line_no,
                cancel_qty,
            )
            .await;
        let (write, left) = match saved {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => return conflict("payload conflict").into_response(),
            Err(error) => return (StatusCode::CONFLICT, format!("{error}")).into_response(),
        };
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "remaining_micros": left,
                "replay": matches!(write, erp_repo::shared_contract::SharedWrite::Replay(_)),
            })),
        )
            .into_response();
    }
    if let Some(delivery_request) = req.delivery_request.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        let saved = repo
            .lookup_sales_delivery(
                tenant_id,
                &req.request_id,
                &req.request_id,
                delivery_request,
            )
            .await;
        let (write, delivery_id, stage, qty) = match saved {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => return conflict("payload conflict").into_response(),
            Err(error) => return (StatusCode::CONFLICT, format!("{error}")).into_response(),
        };
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "delivery_id": delivery_id,
                "stage": stage,
                "qty_micros": qty,
                "replay": matches!(write, erp_repo::shared_contract::SharedWrite::Replay(_)),
            })),
        )
            .into_response();
    }
    if let Some(action) = req.action.as_deref() {
        let delivery_id = match req.delivery_id.as_deref().and_then(|value| uuid::Uuid::parse_str(value).ok()) {
            Some(value) => value,
            None => return bad_request("bad delivery_id").into_response(),
        };
        let warehouse_id = match req.warehouse_id.as_deref().and_then(|value| uuid::Uuid::parse_str(value).ok()) {
            Some(value) => value,
            None => return bad_request("bad warehouse_id").into_response(),
        };
        let location_id = match req.location_id.as_deref().and_then(|value| uuid::Uuid::parse_str(value).ok()) {
            Some(value) => value,
            None => return bad_request("bad location_id").into_response(),
        };
        let sku_id = match req.sku_id.as_deref().and_then(|value| uuid::Uuid::parse_str(value).ok()) {
            Some(value) => value,
            None => return bad_request("bad sku_id").into_response(),
        };
        let saved = repo
            .advance_sales_delivery(erp_repo::shared_contract::SalesDeliveryAdvanceInput { tenant_id, request_id: &req.request_id, payload_hash: &req.request_id, delivery_id, action, carrier_ref: req.carrier_ref.as_deref().unwrap_or(""), warehouse_id, location_id, sku_id, batch_code: req.batch_code.as_deref().unwrap_or("") })
            .await;
        let (write, stage) = match saved {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => return conflict("payload conflict").into_response(),
            Err(error) => return (StatusCode::CONFLICT, format!("{error}")).into_response(),
        };
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "delivery_id": delivery_id,
                "stage": stage,
                "replay": matches!(write, erp_repo::shared_contract::SharedWrite::Replay(_)),
            })),
        )
            .into_response();
    }
    let saved = repo
        .plan_sales_delivery(erp_repo::shared_contract::SalesDeliveryPlanInput { tenant_id, request_id: &req.request_id, payload_hash: &req.request_id, so_id, line_no: req.line_no, expected_version: req.expected_version, qty_micros: req.qty_micros })
        .await;
    let (write, delivery_id) = match saved {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => return conflict("payload conflict").into_response(),
        Err(error) => return (StatusCode::CONFLICT, format!("{error}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "delivery_id": delivery_id,
            "replay": matches!(write, erp_repo::shared_contract::SharedWrite::Replay(_)),
        })),
    )
        .into_response()
}

async fn confirm_sales_balance(
    state: &AppState,
    tenant_id: String,
    ctx: &AuthContext,
    req: &SalesOrderCreateRequest,
    request_id: &str,
) -> Response {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return resp.into_response();
    }
    let Some(document_id) = req.so_id.as_deref().and_then(|value| uuid::Uuid::parse_str(value).ok()) else {
        return bad_request("bad so_id").into_response();
    };
    let partner_code = req.partner_code.as_deref().unwrap_or("").trim();
    let currency = req.currency_code.as_deref().unwrap_or("").trim();
    let due_date = req.due_date.as_deref().unwrap_or("").trim();
    match erp_repo::shared_contract::SharedContractRepo::new(state.db.pool())
        .confirm_document_balance(erp_repo::shared_contract::DocumentBalanceInput { tenant_id, request_id, payload_hash: request_id, document_id, document_kind: "SALES", partner_code, currency, due_date })
        .await
    {
        Ok(Ok((write, balance_id, amount))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "balance_id": balance_id,
                "amount_cent": amount,
                "replay": matches!(write, erp_repo::shared_contract::SharedWrite::Replay(_)),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::CONFLICT, format!("{error}")).into_response(),
    }
}

async fn save_sales_revision_request(
    state: &AppState,
    tenant_id: String,
    ctx: &AuthContext,
    req: &SalesOrderCreateRequest,
    request_id: &str,
) -> Response {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(response) = ensure_permission(ctx, "orders.write") { return response.into_response(); }
    let (lines, so_id, fingerprint) = match revision_lines::prepare(req) {
        Ok(prepared) => prepared,
        Err(error) => return bad_request(error).into_response(),
    };
    let saved = erp_repo::shared_contract::SharedContractRepo::new(state.db.pool())
        .save_sales_revision(erp_repo::shared_contract::TradeSalesRevisionInput { tenant_id, request_id, payload_hash: &fingerprint, so_id, customer_code: req.customer_code.as_deref().unwrap_or("").trim(), expected_version: req.expected_version, action: req.revision_action.as_deref().unwrap_or("SUBMIT").trim(), lines: &lines })
        .await;
    let (write, revision_id, version) = match saved {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => return conflict("payload conflict").into_response(),
        Err(error) => return (StatusCode::CONFLICT, format!("{error}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "revision_id": revision_id,
            "version_no": version,
            "replay": matches!(write, erp_repo::shared_contract::SharedWrite::Replay(_)),
            "shop_required": false,
            "bom_required": false,
        })),
    )
        .into_response()
}

async fn save_scaled_trade_sale(
    state: &AppState,
    tenant_id: String,
    ctx: &AuthContext,
    req: &SalesOrderCreateRequest,
    request_id: &str,
) -> Response {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(ctx, "orders.write") {
        return resp.into_response();
    }
    if req.items.len() != 1 {
        return bad_request("shared trade sale accepts one line").into_response();
    }
    let item = &req.items[0];
    let customer_id = match parse_optional_uuid(req.customer_id.clone(), "customer_id") {
        Ok(Some(value)) => value,
        _ => return bad_request("customer_id 无效").into_response(),
    };
    let sku_id = match item.sku_id.as_deref().and_then(|value| uuid::Uuid::parse_str(value.trim()).ok()) {
        Some(value) => value,
        None => return bad_request("sku_id 无效").into_response(),
    };
    let warehouse_id = match item.warehouse_id.as_deref().and_then(|value| uuid::Uuid::parse_str(value.trim()).ok()) {
        Some(value) => value,
        None => return bad_request("warehouse_id 无效").into_response(),
    };
    let qty_text = item.qty.units();
    let qty_micros = match erp_domain::trade::document_qty_micros(&qty_text) {
        Ok(value) => value,
        Err(_) => return bad_request("bad trade quantity").into_response(),
    };
    let payload = format!("{customer_id}:{sku_id}:{warehouse_id}:{qty_text}:{}", item.price_cent);
    let saved = match erp_repo::shared_contract::SharedContractRepo::new(state.db.pool())
        .save_scaled_trade_sale(erp_repo::shared_contract::ScaledTradeSaleInput { tenant_id, request_id, payload_hash: &payload, customer_id, sku_id, warehouse_id, qty_text: &qty_text, unit_price_cent: item.price_cent, currency: req.currency_code.as_deref().unwrap_or("CNY"), partner_code: req.partner_code.as_deref().unwrap_or("") })
        .await
    {
        Ok(Ok(saved)) => saved,
        Ok(Err(_)) => return conflict("payload conflict").into_response(),
        Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response(),
    };
    let (write, stored_micros, total, replay) = match saved {
        (erp_repo::shared_contract::SharedWrite::Stored(receipt), micros, total) => (receipt, micros, total, false),
        (erp_repo::shared_contract::SharedWrite::Replay(receipt), micros, total) => (receipt, micros, total, true),
    };
    if stored_micros != qty_micros {
        return (StatusCode::INTERNAL_SERVER_ERROR, "stored quantity disagrees").into_response();
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "request_id": write.request_id,
            "status": write.status.as_str(),
            "replay": replay,
            "sales_order_id": write.effect_id,
            "qty_micros": stored_micros,
            "price_cent": item.price_cent,
            "total_cent": total,
            "bom_required": false,
        })),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmScaledSaleRequest {
    request_id: String,
    document_date: String,
}

async fn confirm_scaled_trade_sale_route(
    State(state): State<AppState>,
    Path((tenant_id, sales_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ConfirmScaledSaleRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(tenant_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let sales_order_id = match uuid::Uuid::parse_str(sales_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad sales_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.document_date.trim().len() != 10 {
        return bad_request("bad document_date").into_response();
    }
    let payload = format!("1:{sales_order_id}:{}", req.document_date.trim());
    match erp_repo::shared_contract::SharedContractRepo::new(state.db.pool())
        .confirm_scaled_trade_sale(
            tenant_id,
            &request_id,
            &payload,
            sales_order_id,
            req.document_date.trim(),
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, erp_repo::shared_contract::SharedWrite::Replay(_)),
                "effect_id": sales_order_id,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn create_sales_order(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SalesOrderCreateRequest>,
) -> impl IntoResponse {
    let header_key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let body_key = req.request_id.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let request_id = match (header_key, body_key) {
        (Some(header), Some(body)) if header != body => {
            return conflict("Idempotency-Key disagrees with request_id").into_response();
        }
        (Some(header), _) => Some(header.to_string()),
        (None, Some(body)) => Some(body.to_string()),
        (None, None) => None,
    };
    if let Some(request_id) = request_id {
        if req.confirm_balance.unwrap_or(false) {
            return confirm_sales_balance(&state, tenant_id, &ctx, &req, &request_id).await;
        }
        if req.revision_action.is_some() {
            return save_sales_revision_request(&state, tenant_id, &ctx, &req, &request_id).await;
        }
        return save_scaled_trade_sale(&state, tenant_id, &ctx, &req, &request_id).await;
    }
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }

    if req.sales_order_no.trim().is_empty() {
        return bad_request("sales_order_no 不能为空").into_response();
    }
    if req.source_type.trim().is_empty() {
        return bad_request("source_type 不能为空").into_response();
    }
    if req.items.is_empty() {
        return bad_request("items 不能为空").into_response();
    }

    let shop_id = match parse_optional_uuid(req.shop_id, "shop_id") {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };
    let customer_id = match parse_optional_uuid(req.customer_id, "customer_id") {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };
    let expected_ship_at = match parse_optional_datetime(req.expected_ship_at, "expected_ship_at") {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };

    let items = match parse_sales_order_items(req.items) {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };
    let items_total: i64 = items.iter().map(|item| item.amount_cent).sum();

    let total_amount_cent = req.total_amount_cent.unwrap_or(items_total);
    let discount_amount_cent = req.discount_amount_cent.unwrap_or(0);
    let shipping_amount_cent = req.shipping_amount_cent.unwrap_or(0);
    let receivable_amount_cent = req.receivable_amount_cent.unwrap_or(total_amount_cent);

    let input = SalesOrderCreate {
        sales_order_id: None,
        tenant_id,
        shop_id,
        sales_order_no: req.sales_order_no.trim().to_string(),
        source_type: req.source_type.trim().to_string(),
        source_order_ids: req.source_order_ids.unwrap_or_default(),
        customer_id,
        currency_code: req
            .currency_code
            .and_then(non_empty)
            .unwrap_or_else(|| "CNY".to_string()),
        tax_mode: req.tax_mode.and_then(non_empty),
        sales_order_status: req
            .sales_order_status
            .and_then(non_empty)
            .unwrap_or_else(|| "DRAFT".to_string()),
        delivery_status: req
            .delivery_status
            .and_then(non_empty)
            .unwrap_or_else(|| "NOT_SHIPPED".to_string()),
        invoice_status: req
            .invoice_status
            .and_then(non_empty)
            .unwrap_or_else(|| "NOT_INVOICED".to_string()),
        payment_status: req
            .payment_status
            .and_then(non_empty)
            .unwrap_or_else(|| "UNPAID".to_string()),
        refund_status: req
            .refund_status
            .and_then(non_empty)
            .unwrap_or_else(|| "NONE".to_string()),
        return_status: req.return_status.and_then(non_empty),
        stock_status: req.stock_status.and_then(non_empty),
        total_amount_cent,
        discount_amount_cent,
        shipping_amount_cent,
        receivable_amount_cent,
        paid_amount_cent: req.paid_amount_cent.unwrap_or(0),
        refunded_amount_cent: req.refunded_amount_cent.unwrap_or(0),
        invoice_amount_cent: req.invoice_amount_cent.unwrap_or(0),
        status_flags: req.status_flags.unwrap_or_default(),
        remark: req.remark.and_then(non_empty),
        version: None,
        created_by: Some(ctx.user_id.into()),
        updated_by: Some(ctx.user_id.into()),
        receiver_name: req.receiver_name.and_then(non_empty),
        receiver_phone: req.receiver_phone.and_then(non_empty),
        receiver_address: req.receiver_address.and_then(non_empty),
        receiver_province: req.receiver_province.and_then(non_empty),
        receiver_city: req.receiver_city.and_then(non_empty),
        receiver_district: req.receiver_district.and_then(non_empty),
        receiver_postcode: req.receiver_postcode.and_then(non_empty),
        carrier_code: req.carrier_code.and_then(non_empty),
        logistics_service: req.logistics_service.and_then(non_empty),
        expected_ship_at,
        wms_outbound_id: None,
        wms_outbound_status: None,
        wms_wave_id: None,
        wms_wave_status: None,
        wms_ship_status: None,
        package_count: None,
        waybill_nos: Vec::new(),
        review_status: req.review_status.and_then(non_empty),
        fulfillment_status: req.fulfillment_status.and_then(non_empty),
        exception_status: req.exception_status.and_then(non_empty),
        freeze_reason: req.freeze_reason.and_then(non_empty),
        exception_flags: req.exception_flags.unwrap_or_default(),
        items,
    };

    let repo = SalesOrdersRepo::new(state.db.pool());
    let detail = match repo.create_sales_order(input).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = sales_order_detail_response(detail);
    (StatusCode::OK, Json(response)).into_response()
}

async fn update_sales_order(
    State(state): State<AppState>,
    Path((tenant_id, sales_order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SalesOrderUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let sales_order_id = match uuid::Uuid::parse_str(&sales_order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("sales_order_id 无效").into_response(),
    };

    let shop_id = match parse_optional_uuid(req.shop_id, "shop_id") {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };
    let customer_id = match parse_optional_uuid(req.customer_id, "customer_id") {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };
    let expected_ship_at = match parse_optional_datetime(req.expected_ship_at, "expected_ship_at") {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };

    let items = match parse_optional_sales_order_items(req.items) {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };

    let input = SalesOrderUpdate {
        shop_id,
        sales_order_no: req.sales_order_no.and_then(non_empty),
        source_type: req.source_type.and_then(non_empty),
        source_order_ids: req.source_order_ids,
        customer_id,
        currency_code: req.currency_code.and_then(non_empty),
        tax_mode: req.tax_mode.and_then(non_empty),
        sales_order_status: req.sales_order_status.and_then(non_empty),
        delivery_status: req.delivery_status.and_then(non_empty),
        invoice_status: req.invoice_status.and_then(non_empty),
        payment_status: req.payment_status.and_then(non_empty),
        refund_status: req.refund_status.and_then(non_empty),
        return_status: req.return_status.and_then(non_empty),
        stock_status: req.stock_status.and_then(non_empty),
        total_amount_cent: req.total_amount_cent,
        discount_amount_cent: req.discount_amount_cent,
        shipping_amount_cent: req.shipping_amount_cent,
        receivable_amount_cent: req.receivable_amount_cent,
        paid_amount_cent: req.paid_amount_cent,
        refunded_amount_cent: req.refunded_amount_cent,
        invoice_amount_cent: req.invoice_amount_cent,
        status_flags: req.status_flags,
        remark: req.remark.and_then(non_empty),
        created_by: None,
        updated_by: Some(ctx.user_id.into()),
        receiver_name: req.receiver_name.and_then(non_empty),
        receiver_phone: req.receiver_phone.and_then(non_empty),
        receiver_address: req.receiver_address.and_then(non_empty),
        receiver_province: req.receiver_province.and_then(non_empty),
        receiver_city: req.receiver_city.and_then(non_empty),
        receiver_district: req.receiver_district.and_then(non_empty),
        receiver_postcode: req.receiver_postcode.and_then(non_empty),
        carrier_code: req.carrier_code.and_then(non_empty),
        logistics_service: req.logistics_service.and_then(non_empty),
        expected_ship_at,
        wms_outbound_id: None,
        wms_outbound_status: None,
        wms_wave_id: None,
        wms_wave_status: None,
        wms_ship_status: None,
        package_count: None,
        waybill_nos: None,
        review_status: req.review_status.and_then(non_empty),
        fulfillment_status: req.fulfillment_status.and_then(non_empty),
        exception_status: req.exception_status.and_then(non_empty),
        freeze_reason: req.freeze_reason.and_then(non_empty),
        exception_flags: req.exception_flags,
        items,
    };

    let repo = SalesOrdersRepo::new(state.db.pool());
    let detail = match repo
        .update_sales_order(tenant_id, sales_order_id, input)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("销售单不存在").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = sales_order_detail_response(detail);
    (StatusCode::OK, Json(response)).into_response()
}

async fn update_sales_order_status(
    State(state): State<AppState>,
    Path((tenant_id, sales_order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SalesOrderStatusRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("tenant_id 无效").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let sales_order_id = match uuid::Uuid::parse_str(&sales_order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("sales_order_id 无效").into_response(),
    };

    let update = SalesOrderStatusUpdate {
        sales_order_status: req.sales_order_status.and_then(non_empty),
        delivery_status: req.delivery_status.and_then(non_empty),
        invoice_status: req.invoice_status.and_then(non_empty),
        payment_status: req.payment_status.and_then(non_empty),
        refund_status: req.refund_status.and_then(non_empty),
        return_status: req.return_status.and_then(non_empty),
        stock_status: req.stock_status.and_then(non_empty),
        status_flags: req.status_flags,
        review_status: req.review_status.and_then(non_empty),
        fulfillment_status: req.fulfillment_status.and_then(non_empty),
        exception_status: req.exception_status.and_then(non_empty),
        freeze_reason: req.freeze_reason.and_then(non_empty),
        exception_flags: req.exception_flags,
        updated_by: Some(ctx.user_id.into()),
    };

    let has_updates = update.sales_order_status.is_some()
        || update.delivery_status.is_some()
        || update.invoice_status.is_some()
        || update.payment_status.is_some()
        || update.refund_status.is_some()
        || update.return_status.is_some()
        || update.stock_status.is_some()
        || update.status_flags.is_some()
        || update.review_status.is_some()
        || update.fulfillment_status.is_some()
        || update.exception_status.is_some()
        || update.freeze_reason.is_some()
        || update.exception_flags.is_some();

    if !has_updates {
        return bad_request("状态更新内容为空").into_response();
    }

    let repo = SalesOrdersRepo::new(state.db.pool());
    let order = match repo
        .update_sales_order_status(tenant_id, sales_order_id, update)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("销售单不存在").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = sales_order_response(order);
    (StatusCode::OK, Json(response)).into_response()
}

fn parse_optional_uuid(
    raw: Option<String>,
    label: &str,
) -> Result<Option<uuid::Uuid>, ValidationError> {
    let Some(raw) = raw.and_then(non_empty) else {
        return Ok(None);
    };
    let id = uuid::Uuid::parse_str(&raw).map_err(|_| ValidationError::invalid(label))?;
    Ok(Some(id))
}

fn parse_optional_datetime(
    raw: Option<String>,
    label: &str,
) -> Result<Option<DateTime<Utc>>, ValidationError> {
    let Some(raw) = raw.and_then(non_empty) else {
        return Ok(None);
    };
    let dt = DateTime::parse_from_rfc3339(&raw)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| ValidationError::invalid(label))?;
    Ok(Some(dt))
}

fn parse_sales_order_items(
    items: Vec<SalesOrderItemRequest>,
) -> Result<Vec<SalesOrderItemInput>, ValidationError> {
    let mut parsed = Vec::with_capacity(items.len());
    for item in items {
        let qty = item.qty.legacy_integer().map_err(ValidationError::new)?;
        if qty <= 0 {
            return Err(ValidationError::new("明细数量必须大于 0"));
        }
        let line_id = parse_optional_uuid(item.line_id, "line_id")?;
        let sku_id = parse_optional_uuid(item.sku_id, "sku_id")?;
        let warehouse_id = parse_optional_uuid(item.warehouse_id, "warehouse_id")?;
        let delivery_deadline =
            parse_optional_datetime(item.delivery_deadline, "delivery_deadline")?;
        let discount_cent = item.discount_cent.unwrap_or(0);
        let amount_cent = match item.amount_cent {
            Some(amount) => amount,
            None => item.price_cent.checked_mul(i64::from(qty)).and_then(|amount| amount.checked_sub(discount_cent))
                .ok_or_else(|| ValidationError::new("sales amount overflow"))?,
        };
        parsed.push(SalesOrderItemInput {
            line_id,
            sku_id,
            sku_code: item.sku_code.and_then(non_empty),
            sku_name: item.sku_name.and_then(non_empty),
            qty,
            price_cent: item.price_cent,
            tax_rate: item.tax_rate,
            discount_cent,
            amount_cent,
            warehouse_id,
            delivery_deadline,
            extra: item.extra,
        });
    }
    Ok(parsed)
}

fn parse_optional_sales_order_items(
    items: Option<Vec<SalesOrderItemRequest>>,
) -> Result<Option<Vec<SalesOrderItemInput>>, ValidationError> {
    let Some(items) = items else {
        return Ok(None);
    };
    let parsed = parse_sales_order_items(items)?;
    Ok(Some(parsed))
}

fn sales_order_response(order: SalesOrder) -> SalesOrderResponse {
    SalesOrderResponse {
        sales_order_id: order.sales_order_id,
        shop_id: order.shop_id,
        sales_order_no: order.sales_order_no,
        source_type: order.source_type,
        source_order_ids: order.source_order_ids,
        customer_id: order.customer_id,
        currency_code: order.currency_code,
        tax_mode: order.tax_mode,
        sales_order_status: order.sales_order_status,
        delivery_status: order.delivery_status,
        invoice_status: order.invoice_status,
        payment_status: order.payment_status,
        refund_status: order.refund_status,
        return_status: order.return_status,
        stock_status: order.stock_status,
        total_amount_cent: order.total_amount_cent,
        discount_amount_cent: order.discount_amount_cent,
        shipping_amount_cent: order.shipping_amount_cent,
        receivable_amount_cent: order.receivable_amount_cent,
        paid_amount_cent: order.paid_amount_cent,
        refunded_amount_cent: order.refunded_amount_cent,
        invoice_amount_cent: order.invoice_amount_cent,
        status_flags: order.status_flags,
        remark: order.remark,
        version: order.version,
        created_at: order.created_at,
        updated_at: order.updated_at,
        created_by: order.created_by,
        updated_by: order.updated_by,
        receiver_name: order.receiver_name,
        receiver_phone: order.receiver_phone,
        receiver_address: order.receiver_address,
        receiver_province: order.receiver_province,
        receiver_city: order.receiver_city,
        receiver_district: order.receiver_district,
        receiver_postcode: order.receiver_postcode,
        carrier_code: order.carrier_code,
        logistics_service: order.logistics_service,
        expected_ship_at: order.expected_ship_at,
        wms_outbound_id: order.wms_outbound_id,
        wms_outbound_status: order.wms_outbound_status,
        wms_wave_id: order.wms_wave_id,
        wms_wave_status: order.wms_wave_status,
        wms_ship_status: order.wms_ship_status,
        package_count: order.package_count,
        waybill_nos: order.waybill_nos,
        review_status: order.review_status,
        fulfillment_status: order.fulfillment_status,
        exception_status: order.exception_status,
        freeze_reason: order.freeze_reason,
        exception_flags: order.exception_flags,
    }
}

fn sales_order_item_response(item: SalesOrderItem) -> SalesOrderItemResponse {
    SalesOrderItemResponse {
        line_id: item.line_id,
        sku_id: item.sku_id,
        sku_code: item.sku_code,
        sku_name: item.sku_name,
        qty: item.qty,
        price_cent: item.price_cent,
        tax_rate: item.tax_rate,
        discount_cent: item.discount_cent,
        amount_cent: item.amount_cent,
        warehouse_id: item.warehouse_id,
        delivery_deadline: item.delivery_deadline,
        extra: item.extra,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

fn sales_order_detail_response(detail: SalesOrderDetail) -> SalesOrderDetailResponse {
    SalesOrderDetailResponse {
        order: sales_order_response(detail.order),
        items: detail
            .items
            .into_iter()
            .map(sales_order_item_response)
            .collect(),
    }
}
