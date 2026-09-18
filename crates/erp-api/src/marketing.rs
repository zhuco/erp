use std::str::FromStr;

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use erp_repo::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    marketing_costs::{
        MarketingCost, MarketingCostCreate, MarketingCostListFilters, MarketingCostUpdate,
        MarketingCostsRepo,
    },
    marketing_coupons::{
        MarketingCoupon, MarketingCouponCreate, MarketingCouponListFilters, MarketingCouponUpdate,
        MarketingCouponsRepo,
    },
    marketing_gift_policies::{
        MarketingGiftPoliciesRepo, MarketingGiftPolicy, MarketingGiftPolicyCreate,
        MarketingGiftPolicyListFilters, MarketingGiftPolicyUpdate,
    },
    marketing_postmortems::{
        MarketingPostmortem, MarketingPostmortemCreate, MarketingPostmortemListFilters,
        MarketingPostmortemUpdate, MarketingPostmortemsRepo,
    },
    marketing_price_lists::{
        MarketingPriceItem, MarketingPriceItemCreate, MarketingPriceItemUpdate, MarketingPriceList,
        MarketingPriceListCreate, MarketingPriceListFilters, MarketingPriceListUpdate,
        MarketingPriceListsRepo,
    },
    marketing_promotions::{
        MarketingPromotion, MarketingPromotionCreate, MarketingPromotionListFilters,
        MarketingPromotionUpdate, MarketingPromotionsRepo,
    },
};
use erp_types::{PermissionCode, TenantId};

use crate::{
    AppState, AuthContext, RequestContext, bad_request, ensure_permission, ensure_tenant_access,
    not_found,
};

#[derive(Deserialize)]
pub(crate) struct PriceListListQuery {
    keyword: Option<String>,
    status: Option<String>,
    channel_id: Option<String>,
    currency: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct PriceListListResponse {
    items: Vec<MarketingPriceList>,
    total: i64,
}

#[derive(Serialize)]
pub(crate) struct PriceListDetailResponse {
    price_list: MarketingPriceList,
    items: Vec<MarketingPriceItem>,
}

#[derive(Deserialize)]
pub(crate) struct PriceListCreateRequest {
    name: String,
    channel_id: Option<String>,
    currency: Option<String>,
    includes_tax: Option<bool>,
    status: Option<String>,
    version: Option<i32>,
    effective_from: Option<DateTime<Utc>>,
    effective_to: Option<DateTime<Utc>>,
    notes: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct PriceListUpdateRequest {
    name: Option<String>,
    channel_id: Option<String>,
    currency: Option<String>,
    includes_tax: Option<bool>,
    status: Option<String>,
    version: Option<i32>,
    effective_from: Option<DateTime<Utc>>,
    effective_to: Option<DateTime<Utc>>,
    notes: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct PriceItemCreateRequest {
    sku_id: String,
    base_price_cent: Option<i64>,
    channel_price_cent: Option<i64>,
    member_price_cent: Option<i64>,
}

#[derive(Deserialize)]
pub(crate) struct PriceItemUpdateRequest {
    base_price_cent: Option<i64>,
    channel_price_cent: Option<i64>,
    member_price_cent: Option<i64>,
}

#[derive(Deserialize)]
pub(crate) struct PromotionListQuery {
    keyword: Option<String>,
    status: Option<String>,
    promo_type: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct PromotionListResponse {
    items: Vec<MarketingPromotion>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct PromotionCreateRequest {
    name: String,
    promo_type: String,
    status: Option<String>,
    priority: Option<i32>,
    exclusive: Option<bool>,
    scope_type: Option<String>,
    scope_json: Option<Value>,
    rule_json: Option<Value>,
    start_at: Option<DateTime<Utc>>,
    end_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub(crate) struct PromotionUpdateRequest {
    name: Option<String>,
    promo_type: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
    exclusive: Option<bool>,
    scope_type: Option<String>,
    scope_json: Option<Value>,
    rule_json: Option<Value>,
    start_at: Option<DateTime<Utc>>,
    end_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub(crate) struct GiftPolicyListQuery {
    keyword: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct GiftPolicyListResponse {
    items: Vec<MarketingGiftPolicy>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct GiftPolicyCreateRequest {
    name: String,
    status: Option<String>,
    strategy: Option<String>,
    probability: Option<f64>,
    scope_json: Option<Value>,
    gift_pool: Option<Value>,
    start_at: Option<DateTime<Utc>>,
    end_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub(crate) struct GiftPolicyUpdateRequest {
    name: Option<String>,
    status: Option<String>,
    strategy: Option<String>,
    probability: Option<f64>,
    scope_json: Option<Value>,
    gift_pool: Option<Value>,
    start_at: Option<DateTime<Utc>>,
    end_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub(crate) struct CouponListQuery {
    keyword: Option<String>,
    status: Option<String>,
    discount_type: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct CouponListResponse {
    items: Vec<MarketingCoupon>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct CouponCreateRequest {
    name: String,
    code_prefix: Option<String>,
    discount_type: String,
    discount_value: f64,
    max_usage: Option<i32>,
    per_user_limit: Option<i32>,
    status: Option<String>,
    rules_json: Option<Value>,
    start_at: Option<DateTime<Utc>>,
    end_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub(crate) struct CouponUpdateRequest {
    name: Option<String>,
    code_prefix: Option<String>,
    discount_type: Option<String>,
    discount_value: Option<f64>,
    max_usage: Option<i32>,
    per_user_limit: Option<i32>,
    status: Option<String>,
    rules_json: Option<Value>,
    start_at: Option<DateTime<Utc>>,
    end_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub(crate) struct CostListQuery {
    keyword: Option<String>,
    status: Option<String>,
    channel_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct CostListResponse {
    items: Vec<MarketingCost>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct CostCreateRequest {
    name: String,
    channel_id: Option<String>,
    amount_cent: i64,
    currency: Option<String>,
    allocation_rule: Option<Value>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct CostUpdateRequest {
    name: Option<String>,
    channel_id: Option<String>,
    amount_cent: Option<i64>,
    currency: Option<String>,
    allocation_rule: Option<Value>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct PostmortemListQuery {
    keyword: Option<String>,
    promotion_id: Option<String>,
    channel_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct PostmortemListResponse {
    items: Vec<MarketingPostmortem>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct PostmortemCreateRequest {
    name: String,
    promotion_id: Option<String>,
    channel_id: Option<String>,
    summary: Option<String>,
    metrics_json: Option<Value>,
}

#[derive(Deserialize)]
pub(crate) struct PostmortemUpdateRequest {
    name: Option<String>,
    promotion_id: Option<String>,
    channel_id: Option<String>,
    summary: Option<String>,
    metrics_json: Option<Value>,
}

fn parse_uuid_optional(
    raw: Option<String>,
) -> Result<Option<uuid::Uuid>, (StatusCode, Json<crate::ErrorResponse>)> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    match uuid::Uuid::parse_str(raw) {
        Ok(value) => Ok(Some(value)),
        Err(_) => Err(bad_request("bad uuid")),
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub(super) async fn list_price_lists(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PriceListListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPriceRead.as_str()) {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let channel_id = match parse_uuid_optional(query.channel_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = MarketingPriceListsRepo::new(state.db.pool());
    let result = match repo
        .list_price_lists(
            tenant_id,
            MarketingPriceListFilters {
                keyword: normalize_optional_text(query.keyword),
                status: normalize_optional_text(query.status),
                channel_id,
                currency: normalize_optional_text(query.currency),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    let response = PriceListListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn get_price_list(
    State(state): State<AppState>,
    Path((tenant_id, price_list_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let price_list_id = match uuid::Uuid::parse_str(&price_list_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad price_list_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPriceRead.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingPriceListsRepo::new(state.db.pool());
    let price_list = match repo.get_price_list(tenant_id, price_list_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("price list not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let items = match repo.list_price_items(tenant_id, price_list_id).await {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(PriceListDetailResponse { price_list, items }),
    )
        .into_response()
}

pub(super) async fn create_price_list(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PriceListCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPriceWrite.as_str()) {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let channel_id = match parse_uuid_optional(req.channel_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = MarketingPriceListsRepo::new(state.db.pool());
    let price_list = match repo
        .create_price_list(MarketingPriceListCreate {
            tenant_id,
            name: name.to_string(),
            channel_id,
            currency: req.currency.unwrap_or_else(|| "CNY".to_string()),
            includes_tax: req.includes_tax.unwrap_or(true),
            status: req.status.unwrap_or_else(|| "DRAFT".to_string()),
            version: req.version.unwrap_or(1),
            effective_from: req.effective_from,
            effective_to: req.effective_to,
            notes: normalize_optional_text(req.notes),
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.PRICE_LIST.CREATE",
        "marketing_price_list",
        price_list.price_list_id.to_string(),
        None,
        Some(serde_json::to_value(&price_list).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(price_list)).into_response()
}

pub(super) async fn update_price_list(
    State(state): State<AppState>,
    Path((tenant_id, price_list_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PriceListUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let price_list_id = match uuid::Uuid::parse_str(&price_list_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad price_list_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPriceWrite.as_str()) {
        return resp.into_response();
    }

    let channel_id = match parse_uuid_optional(req.channel_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = MarketingPriceListsRepo::new(state.db.pool());
    let before = match repo.get_price_list(tenant_id, price_list_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("price list not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let updated = match repo
        .update_price_list(
            tenant_id,
            price_list_id,
            MarketingPriceListUpdate {
                name: normalize_optional_text(req.name),
                channel_id,
                currency: normalize_optional_text(req.currency),
                includes_tax: req.includes_tax,
                status: normalize_optional_text(req.status),
                version: req.version,
                effective_from: req.effective_from,
                effective_to: req.effective_to,
                notes: normalize_optional_text(req.notes),
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("price list not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.PRICE_LIST.UPDATE",
        "marketing_price_list",
        updated.price_list_id.to_string(),
        Some(serde_json::to_value(&before).unwrap_or(Value::Null)),
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn delete_price_list(
    State(state): State<AppState>,
    Path((tenant_id, price_list_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let price_list_id = match uuid::Uuid::parse_str(&price_list_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad price_list_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPriceWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingPriceListsRepo::new(state.db.pool());
    let deleted = match repo.delete_price_list(tenant_id, price_list_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("price list not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.PRICE_LIST.DELETE",
        "marketing_price_list",
        deleted.price_list_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(super) async fn create_price_item(
    State(state): State<AppState>,
    Path((tenant_id, price_list_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PriceItemCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let price_list_id = match uuid::Uuid::parse_str(&price_list_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad price_list_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPriceWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingPriceListsRepo::new(state.db.pool());
    let item = match repo
        .create_price_item(MarketingPriceItemCreate {
            tenant_id,
            price_list_id,
            sku_id,
            base_price_cent: req.base_price_cent,
            channel_price_cent: req.channel_price_cent,
            member_price_cent: req.member_price_cent,
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.PRICE_ITEM.CREATE",
        "marketing_price_item",
        item.item_id.to_string(),
        None,
        Some(serde_json::to_value(&item).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(item)).into_response()
}

pub(super) async fn update_price_item(
    State(state): State<AppState>,
    Path((tenant_id, item_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PriceItemUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let item_id = match uuid::Uuid::parse_str(&item_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad item_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPriceWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingPriceListsRepo::new(state.db.pool());
    let updated = match repo
        .update_price_item(
            tenant_id,
            item_id,
            MarketingPriceItemUpdate {
                base_price_cent: req.base_price_cent,
                channel_price_cent: req.channel_price_cent,
                member_price_cent: req.member_price_cent,
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("price item not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.PRICE_ITEM.UPDATE",
        "marketing_price_item",
        updated.item_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn delete_price_item(
    State(state): State<AppState>,
    Path((tenant_id, item_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let item_id = match uuid::Uuid::parse_str(&item_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad item_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPriceWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingPriceListsRepo::new(state.db.pool());
    let deleted = match repo.delete_price_item(tenant_id, item_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("price item not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.PRICE_ITEM.DELETE",
        "marketing_price_item",
        deleted.item_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(super) async fn list_promotions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PromotionListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPromotionRead.as_str()) {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = MarketingPromotionsRepo::new(state.db.pool());
    let result = match repo
        .list_promotions(
            tenant_id,
            MarketingPromotionListFilters {
                keyword: normalize_optional_text(query.keyword),
                status: normalize_optional_text(query.status),
                promo_type: normalize_optional_text(query.promo_type),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(PromotionListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(super) async fn create_promotion(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PromotionCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPromotionWrite.as_str()) {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let promo_type = req.promo_type.trim();
    if promo_type.is_empty() {
        return bad_request("promo_type required").into_response();
    }

    let repo = MarketingPromotionsRepo::new(state.db.pool());
    let promotion = match repo
        .create_promotion(MarketingPromotionCreate {
            tenant_id,
            name: name.to_string(),
            promo_type: promo_type.to_string(),
            status: req.status.unwrap_or_else(|| "DRAFT".to_string()),
            priority: req.priority.unwrap_or(0),
            exclusive: req.exclusive.unwrap_or(false),
            scope_type: req.scope_type.unwrap_or_else(|| "SKU".to_string()),
            scope_json: req
                .scope_json
                .unwrap_or_else(|| Value::Object(Default::default())),
            rule_json: req
                .rule_json
                .unwrap_or_else(|| Value::Object(Default::default())),
            start_at: req.start_at,
            end_at: req.end_at,
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.PROMOTION.CREATE",
        "marketing_promotion",
        promotion.promotion_id.to_string(),
        None,
        Some(serde_json::to_value(&promotion).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(promotion)).into_response()
}

pub(super) async fn update_promotion(
    State(state): State<AppState>,
    Path((tenant_id, promotion_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PromotionUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let promotion_id = match uuid::Uuid::parse_str(&promotion_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad promotion_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPromotionWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingPromotionsRepo::new(state.db.pool());
    let updated = match repo
        .update_promotion(
            tenant_id,
            promotion_id,
            MarketingPromotionUpdate {
                name: normalize_optional_text(req.name),
                promo_type: normalize_optional_text(req.promo_type),
                status: normalize_optional_text(req.status),
                priority: req.priority,
                exclusive: req.exclusive,
                scope_type: normalize_optional_text(req.scope_type),
                scope_json: req.scope_json,
                rule_json: req.rule_json,
                start_at: req.start_at,
                end_at: req.end_at,
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("promotion not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.PROMOTION.UPDATE",
        "marketing_promotion",
        updated.promotion_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn delete_promotion(
    State(state): State<AppState>,
    Path((tenant_id, promotion_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let promotion_id = match uuid::Uuid::parse_str(&promotion_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad promotion_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPromotionWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingPromotionsRepo::new(state.db.pool());
    let deleted = match repo.delete_promotion(tenant_id, promotion_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("promotion not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.PROMOTION.DELETE",
        "marketing_promotion",
        deleted.promotion_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(super) async fn list_gift_policies(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<GiftPolicyListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingGiftRead.as_str()) {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = MarketingGiftPoliciesRepo::new(state.db.pool());
    let result = match repo
        .list_policies(
            tenant_id,
            MarketingGiftPolicyListFilters {
                keyword: normalize_optional_text(query.keyword),
                status: normalize_optional_text(query.status),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(GiftPolicyListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(super) async fn create_gift_policy(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<GiftPolicyCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingGiftWrite.as_str()) {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }

    let repo = MarketingGiftPoliciesRepo::new(state.db.pool());
    let policy = match repo
        .create_policy(MarketingGiftPolicyCreate {
            tenant_id,
            name: name.to_string(),
            status: req.status.unwrap_or_else(|| "DRAFT".to_string()),
            strategy: req.strategy.unwrap_or_else(|| "RANDOM".to_string()),
            probability: req.probability,
            scope_json: req
                .scope_json
                .unwrap_or_else(|| Value::Object(Default::default())),
            gift_pool: req.gift_pool.unwrap_or_else(|| Value::Array(Vec::new())),
            start_at: req.start_at,
            end_at: req.end_at,
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.GIFT_POLICY.CREATE",
        "marketing_gift_policy",
        policy.policy_id.to_string(),
        None,
        Some(serde_json::to_value(&policy).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(policy)).into_response()
}

pub(super) async fn update_gift_policy(
    State(state): State<AppState>,
    Path((tenant_id, policy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<GiftPolicyUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let policy_id = match uuid::Uuid::parse_str(&policy_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad policy_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingGiftWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingGiftPoliciesRepo::new(state.db.pool());
    let updated = match repo
        .update_policy(
            tenant_id,
            policy_id,
            MarketingGiftPolicyUpdate {
                name: normalize_optional_text(req.name),
                status: normalize_optional_text(req.status),
                strategy: normalize_optional_text(req.strategy),
                probability: req.probability,
                scope_json: req.scope_json,
                gift_pool: req.gift_pool,
                start_at: req.start_at,
                end_at: req.end_at,
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("gift policy not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.GIFT_POLICY.UPDATE",
        "marketing_gift_policy",
        updated.policy_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn delete_gift_policy(
    State(state): State<AppState>,
    Path((tenant_id, policy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let policy_id = match uuid::Uuid::parse_str(&policy_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad policy_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingGiftWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingGiftPoliciesRepo::new(state.db.pool());
    let deleted = match repo.delete_policy(tenant_id, policy_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("gift policy not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.GIFT_POLICY.DELETE",
        "marketing_gift_policy",
        deleted.policy_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(super) async fn list_coupons(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CouponListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingCouponRead.as_str()) {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = MarketingCouponsRepo::new(state.db.pool());
    let result = match repo
        .list_coupons(
            tenant_id,
            MarketingCouponListFilters {
                keyword: normalize_optional_text(query.keyword),
                status: normalize_optional_text(query.status),
                discount_type: normalize_optional_text(query.discount_type),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(CouponListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(super) async fn create_coupon(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<CouponCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingCouponWrite.as_str()) {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let discount_type = req.discount_type.trim();
    if discount_type.is_empty() {
        return bad_request("discount_type required").into_response();
    }

    let repo = MarketingCouponsRepo::new(state.db.pool());
    let coupon = match repo
        .create_coupon(MarketingCouponCreate {
            tenant_id,
            name: name.to_string(),
            code_prefix: normalize_optional_text(req.code_prefix),
            discount_type: discount_type.to_string(),
            discount_value: req.discount_value,
            max_usage: req.max_usage.unwrap_or(0),
            per_user_limit: req.per_user_limit.unwrap_or(0),
            status: req.status.unwrap_or_else(|| "DRAFT".to_string()),
            rules_json: req
                .rules_json
                .unwrap_or_else(|| Value::Object(Default::default())),
            start_at: req.start_at,
            end_at: req.end_at,
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.COUPON.CREATE",
        "marketing_coupon",
        coupon.coupon_id.to_string(),
        None,
        Some(serde_json::to_value(&coupon).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(coupon)).into_response()
}

pub(super) async fn update_coupon(
    State(state): State<AppState>,
    Path((tenant_id, coupon_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<CouponUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let coupon_id = match uuid::Uuid::parse_str(&coupon_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad coupon_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingCouponWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingCouponsRepo::new(state.db.pool());
    let updated = match repo
        .update_coupon(
            tenant_id,
            coupon_id,
            MarketingCouponUpdate {
                name: normalize_optional_text(req.name),
                code_prefix: normalize_optional_text(req.code_prefix),
                discount_type: normalize_optional_text(req.discount_type),
                discount_value: req.discount_value,
                max_usage: req.max_usage,
                per_user_limit: req.per_user_limit,
                status: normalize_optional_text(req.status),
                rules_json: req.rules_json,
                start_at: req.start_at,
                end_at: req.end_at,
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("coupon not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.COUPON.UPDATE",
        "marketing_coupon",
        updated.coupon_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn delete_coupon(
    State(state): State<AppState>,
    Path((tenant_id, coupon_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let coupon_id = match uuid::Uuid::parse_str(&coupon_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad coupon_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingCouponWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingCouponsRepo::new(state.db.pool());
    let deleted = match repo.delete_coupon(tenant_id, coupon_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("coupon not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.COUPON.DELETE",
        "marketing_coupon",
        deleted.coupon_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(super) async fn list_costs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CostListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingCostRead.as_str()) {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let channel_id = match parse_uuid_optional(query.channel_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = MarketingCostsRepo::new(state.db.pool());
    let result = match repo
        .list_costs(
            tenant_id,
            MarketingCostListFilters {
                keyword: normalize_optional_text(query.keyword),
                status: normalize_optional_text(query.status),
                channel_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(CostListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(super) async fn create_cost(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<CostCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingCostWrite.as_str()) {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let channel_id = match parse_uuid_optional(req.channel_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = MarketingCostsRepo::new(state.db.pool());
    let cost = match repo
        .create_cost(MarketingCostCreate {
            tenant_id,
            name: name.to_string(),
            channel_id,
            amount_cent: req.amount_cent,
            currency: req.currency.unwrap_or_else(|| "CNY".to_string()),
            allocation_rule: req
                .allocation_rule
                .unwrap_or_else(|| Value::Object(Default::default())),
            period_start: req.period_start,
            period_end: req.period_end,
            status: req.status.unwrap_or_else(|| "ACTIVE".to_string()),
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.COST.CREATE",
        "marketing_cost",
        cost.cost_id.to_string(),
        None,
        Some(serde_json::to_value(&cost).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(cost)).into_response()
}

pub(super) async fn update_cost(
    State(state): State<AppState>,
    Path((tenant_id, cost_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<CostUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let cost_id = match uuid::Uuid::parse_str(&cost_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad cost_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingCostWrite.as_str()) {
        return resp.into_response();
    }

    let channel_id = match parse_uuid_optional(req.channel_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = MarketingCostsRepo::new(state.db.pool());
    let updated = match repo
        .update_cost(
            tenant_id,
            cost_id,
            MarketingCostUpdate {
                name: normalize_optional_text(req.name),
                channel_id,
                amount_cent: req.amount_cent,
                currency: normalize_optional_text(req.currency),
                allocation_rule: req.allocation_rule,
                period_start: req.period_start,
                period_end: req.period_end,
                status: normalize_optional_text(req.status),
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("cost not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.COST.UPDATE",
        "marketing_cost",
        updated.cost_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn delete_cost(
    State(state): State<AppState>,
    Path((tenant_id, cost_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let cost_id = match uuid::Uuid::parse_str(&cost_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad cost_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingCostWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingCostsRepo::new(state.db.pool());
    let deleted = match repo.delete_cost(tenant_id, cost_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("cost not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.COST.DELETE",
        "marketing_cost",
        deleted.cost_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(super) async fn list_postmortems(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PostmortemListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPostmortemRead.as_str()) {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let promotion_id = match parse_uuid_optional(query.promotion_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let channel_id = match parse_uuid_optional(query.channel_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = MarketingPostmortemsRepo::new(state.db.pool());
    let result = match repo
        .list_postmortems(
            tenant_id,
            MarketingPostmortemListFilters {
                keyword: normalize_optional_text(query.keyword),
                promotion_id,
                channel_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(PostmortemListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(super) async fn create_postmortem(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PostmortemCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPostmortemWrite.as_str()) {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let promotion_id = match parse_uuid_optional(req.promotion_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let channel_id = match parse_uuid_optional(req.channel_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = MarketingPostmortemsRepo::new(state.db.pool());
    let postmortem = match repo
        .create_postmortem(MarketingPostmortemCreate {
            tenant_id,
            name: name.to_string(),
            promotion_id,
            channel_id,
            summary: normalize_optional_text(req.summary),
            metrics_json: req
                .metrics_json
                .unwrap_or_else(|| Value::Object(Default::default())),
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.POSTMORTEM.CREATE",
        "marketing_postmortem",
        postmortem.postmortem_id.to_string(),
        None,
        Some(serde_json::to_value(&postmortem).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(postmortem)).into_response()
}

pub(super) async fn update_postmortem(
    State(state): State<AppState>,
    Path((tenant_id, postmortem_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PostmortemUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let postmortem_id = match uuid::Uuid::parse_str(&postmortem_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad postmortem_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPostmortemWrite.as_str()) {
        return resp.into_response();
    }

    let promotion_id = match parse_uuid_optional(req.promotion_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let channel_id = match parse_uuid_optional(req.channel_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = MarketingPostmortemsRepo::new(state.db.pool());
    let updated = match repo
        .update_postmortem(
            tenant_id,
            postmortem_id,
            MarketingPostmortemUpdate {
                name: normalize_optional_text(req.name),
                promotion_id,
                channel_id,
                summary: normalize_optional_text(req.summary),
                metrics_json: req.metrics_json,
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("postmortem not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.POSTMORTEM.UPDATE",
        "marketing_postmortem",
        updated.postmortem_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn delete_postmortem(
    State(state): State<AppState>,
    Path((tenant_id, postmortem_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let postmortem_id = match uuid::Uuid::parse_str(&postmortem_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad postmortem_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::MarketingPostmortemWrite.as_str()) {
        return resp.into_response();
    }

    let repo = MarketingPostmortemsRepo::new(state.db.pool());
    let deleted = match repo.delete_postmortem(tenant_id, postmortem_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("postmortem not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_marketing_audit(
        &state,
        &ctx,
        &req_ctx,
        "MARKETING.POSTMORTEM.DELETE",
        "marketing_postmortem",
        deleted.postmortem_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

#[allow(clippy::too_many_arguments)]
async fn record_marketing_audit(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    action: &str,
    entity_type: &str,
    entity_id: String,
    before: Option<Value>,
    after: Option<Value>,
) {
    let repo = AuditLogsRepo::new(state.db.pool());
    let _ = repo
        .insert(AuditLogCreate {
            tenant_id: ctx.tenant_id,
            user_id: Some(ctx.user_id),
            action: action.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: Some(entity_id),
            before_json: before,
            after_json: after,
            ip: req_ctx.ip.clone(),
            ua: req_ctx.user_agent.clone(),
        })
        .await;
}
