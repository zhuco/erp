use super::*;
use erp_repo::planning::{
    DemandForecast, DemandForecastUpsert, PlanExecution, PlanningRepo, PromoStockingCampaign,
    PromoStockingGenerateInput, PurchasePlan, ReplenishmentRule, ReplenishmentRuleUpsert,
    TransferSuggestion,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/tenants/:tenant_id/planning/forecasts",
            get(list_demand_forecasts).post(upsert_demand_forecast),
        )
        .route(
            "/v1/tenants/:tenant_id/planning/replenishment-rules",
            get(list_replenishment_rules).post(upsert_replenishment_rule),
        )
        .route(
            "/v1/tenants/:tenant_id/planning/purchase-plans",
            get(list_purchase_plans),
        )
        .route(
            "/v1/tenants/:tenant_id/planning/purchase-plans/generate",
            post(generate_purchase_plans),
        )
        .route(
            "/v1/tenants/:tenant_id/planning/transfer-suggestions",
            get(list_transfer_suggestions),
        )
        .route(
            "/v1/tenants/:tenant_id/planning/transfer-suggestions/generate",
            post(generate_transfer_suggestions),
        )
        .route(
            "/v1/tenants/:tenant_id/planning/promo-stocking",
            get(list_promo_stocking),
        )
        .route(
            "/v1/tenants/:tenant_id/planning/promo-stocking/generate",
            post(generate_promo_stocking),
        )
        .route(
            "/v1/tenants/:tenant_id/planning/plan-executions",
            get(list_plan_executions),
        )
}

#[derive(Debug, Serialize)]
struct DemandForecastListResponse {
    items: Vec<DemandForecast>,
    total: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DemandForecastUpsertRequest {
    sku_id: String,
    sku_code: Option<String>,
    warehouse_id: String,
    forecast_qty: i64,
    model: Option<String>,
    confidence: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReplenishmentRuleListResponse {
    items: Vec<ReplenishmentRule>,
    total: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReplenishmentRuleUpsertRequest {
    warehouse_id: String,
    category: String,
    safety_stock_days: i32,
    replenishment_cycle_days: i32,
    min_replenishment_qty: i64,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct PurchasePlanListResponse {
    items: Vec<PurchasePlan>,
    total: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PurchasePlanGenerateRequest {
    trigger: Option<String>,
}

#[derive(Debug, Serialize)]
struct TransferSuggestionListResponse {
    items: Vec<TransferSuggestion>,
    total: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TransferSuggestionGenerateRequest {
    strategy: Option<String>,
}

#[derive(Debug, Serialize)]
struct PromoStockingListResponse {
    items: Vec<PromoStockingCampaign>,
    total: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PromoStockingGenerateRequest {
    campaign_name: String,
    period_start: chrono::NaiveDate,
    period_end: chrono::NaiveDate,
    locked_budget_cent: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PlanExecutionListResponse {
    items: Vec<PlanExecution>,
    total: i64,
}

fn parse_tenant_or_bad_request(
    tenant_id: &str,
) -> Result<TenantId, (StatusCode, Json<crate::ErrorResponse>)> {
    TenantId::from_str(tenant_id).map_err(|_| bad_request("bad tenant_id"))
}

fn ensure_planning_read_permission(
    ctx: &AuthContext,
) -> Result<(), (StatusCode, Json<crate::ErrorResponse>)> {
    ensure_permission(ctx, PermissionCode::WmsInventoryRead.as_str())
}

fn ensure_planning_write_permission(
    ctx: &AuthContext,
) -> Result<(), (StatusCode, Json<crate::ErrorResponse>)> {
    ensure_permission(ctx, PermissionCode::WmsInventoryWrite.as_str())
}

pub(super) async fn list_demand_forecasts(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_read_permission(&ctx) {
        return resp.into_response();
    }

    let repo = PlanningRepo::new(state.db.pool());
    let items = match repo.list_demand_forecasts(tenant_id).await {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(DemandForecastListResponse {
            total: items.len() as i64,
            items,
        }),
    )
        .into_response()
}

pub(super) async fn upsert_demand_forecast(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DemandForecastUpsertRequest>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_write_permission(&ctx) {
        return resp.into_response();
    }

    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    if req.forecast_qty < 0 {
        return bad_request("forecast_qty must be >= 0").into_response();
    }
    let model = req
        .model
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "SEASONAL".to_string());
    let confidence = req
        .confidence
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "MEDIUM".to_string());
    let version = req
        .version
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "LATEST".to_string());

    let repo = PlanningRepo::new(state.db.pool());
    let forecast = match repo
        .upsert_demand_forecast(DemandForecastUpsert {
            tenant_id,
            sku_id,
            sku_code: req.sku_code.map(|value| value.trim().to_string()),
            warehouse_id,
            forecast_qty: req.forecast_qty,
            model,
            confidence,
            version,
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (StatusCode::OK, Json(forecast)).into_response()
}

pub(super) async fn list_replenishment_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_read_permission(&ctx) {
        return resp.into_response();
    }

    let repo = PlanningRepo::new(state.db.pool());
    let items = match repo.list_replenishment_rules(tenant_id).await {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(ReplenishmentRuleListResponse {
            total: items.len() as i64,
            items,
        }),
    )
        .into_response()
}

pub(super) async fn upsert_replenishment_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ReplenishmentRuleUpsertRequest>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_write_permission(&ctx) {
        return resp.into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let category = req.category.trim().to_string();
    if category.is_empty() {
        return bad_request("category required").into_response();
    }
    if req.safety_stock_days <= 0 {
        return bad_request("safety_stock_days must be > 0").into_response();
    }
    if req.replenishment_cycle_days <= 0 {
        return bad_request("replenishment_cycle_days must be > 0").into_response();
    }
    if req.min_replenishment_qty <= 0 {
        return bad_request("min_replenishment_qty must be > 0").into_response();
    }

    let repo = PlanningRepo::new(state.db.pool());
    let rule = match repo
        .upsert_replenishment_rule(ReplenishmentRuleUpsert {
            tenant_id,
            warehouse_id,
            category,
            safety_stock_days: req.safety_stock_days,
            replenishment_cycle_days: req.replenishment_cycle_days,
            min_replenishment_qty: req.min_replenishment_qty,
            status: req
                .status
                .map(|value| value.trim().to_uppercase())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "ACTIVE".to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (StatusCode::OK, Json(rule)).into_response()
}

pub(super) async fn list_purchase_plans(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_read_permission(&ctx) {
        return resp.into_response();
    }

    let repo = PlanningRepo::new(state.db.pool());
    let items = match repo.list_purchase_plans(tenant_id).await {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(PurchasePlanListResponse {
            total: items.len() as i64,
            items,
        }),
    )
        .into_response()
}

pub(super) async fn generate_purchase_plans(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PurchasePlanGenerateRequest>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_write_permission(&ctx) {
        return resp.into_response();
    }

    let repo = PlanningRepo::new(state.db.pool());
    let items = match repo.generate_purchase_plans(tenant_id, req.trigger).await {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(PurchasePlanListResponse {
            total: items.len() as i64,
            items,
        }),
    )
        .into_response()
}

pub(super) async fn list_transfer_suggestions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_read_permission(&ctx) {
        return resp.into_response();
    }

    let repo = PlanningRepo::new(state.db.pool());
    let items = match repo.list_transfer_suggestions(tenant_id).await {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(TransferSuggestionListResponse {
            total: items.len() as i64,
            items,
        }),
    )
        .into_response()
}

pub(super) async fn generate_transfer_suggestions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TransferSuggestionGenerateRequest>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_write_permission(&ctx) {
        return resp.into_response();
    }

    let repo = PlanningRepo::new(state.db.pool());
    let items = match repo
        .generate_transfer_suggestions(tenant_id, req.strategy)
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(TransferSuggestionListResponse {
            total: items.len() as i64,
            items,
        }),
    )
        .into_response()
}

pub(super) async fn list_promo_stocking(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_read_permission(&ctx) {
        return resp.into_response();
    }

    let repo = PlanningRepo::new(state.db.pool());
    let items = match repo.list_promo_stocking(tenant_id).await {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(PromoStockingListResponse {
            total: items.len() as i64,
            items,
        }),
    )
        .into_response()
}

pub(super) async fn generate_promo_stocking(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PromoStockingGenerateRequest>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_write_permission(&ctx) {
        return resp.into_response();
    }

    let campaign_name = req.campaign_name.trim().to_string();
    if campaign_name.is_empty() {
        return bad_request("campaign_name required").into_response();
    }
    if req.period_end < req.period_start {
        return bad_request("period_end must be >= period_start").into_response();
    }

    let repo = PlanningRepo::new(state.db.pool());
    let campaign = match repo
        .generate_promo_stocking(PromoStockingGenerateInput {
            tenant_id,
            campaign_name,
            period_start: req.period_start,
            period_end: req.period_end,
            locked_budget_cent: req.locked_budget_cent.unwrap_or(0),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    (StatusCode::OK, Json(campaign)).into_response()
}

pub(super) async fn list_plan_executions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant_or_bad_request(&tenant_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_planning_read_permission(&ctx) {
        return resp.into_response();
    }

    let repo = PlanningRepo::new(state.db.pool());
    let items = match repo.list_plan_executions(tenant_id).await {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(PlanExecutionListResponse {
            total: items.len() as i64,
            items,
        }),
    )
        .into_response()
}
