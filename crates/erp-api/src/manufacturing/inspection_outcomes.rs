//! Authorized measurement history, explicit reinspection and authenticated review.
use super::*;
use erp_repo::shared_contract::{
    InspectionActor, InspectionOutcomeWrite, InspectionReadingInput, InspectionReviewInput,
};

pub(super) fn actor(ctx: &AuthContext) -> InspectionActor {
    InspectionActor::Authenticated {
        tenant_id: ctx.tenant_id,
        user_id: ctx.user_id,
    }
}

pub(super) fn quality_tenant(
    tenant_id: &str,
    ctx: &AuthContext,
) -> Result<TenantId, (StatusCode, Json<ErrorResponse>)> {
    let tenant = TenantId::from_str(tenant_id).map_err(|_| bad_request("bad tenant_id"))?;
    ensure_tenant_access(ctx, tenant)?;
    Ok(tenant)
}

pub(super) fn require_quality(
    ctx: &AuthContext,
    permission: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if ctx.is_admin
        || ctx
            .perms
            .iter()
            .any(|p| p == permission || p == "*" || p == "quality.*")
    {
        Ok(())
    } else {
        Err(forbidden("quality permission required"))
    }
}

pub(super) fn outcome_response(
    request: &str,
    result: InspectionOutcomeWrite,
    key: &str,
) -> Response {
    let mut response = serde_json::json!({"request_id":request,"status":"SUCCEEDED",
    "replay":matches!(result.write, SharedWrite::Replay(_)),
    "effect_id":match &result.write {
        SharedWrite::Stored(receipt)|SharedWrite::Replay(receipt)=>receipt.effect_id.clone()
    }});
    response[key] = result.snapshot;
    (StatusCode::OK, Json(response)).into_response()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InspectionReinspectionRequest {
    request_id: String,
    item_code: String,
    reading_qty: Option<String>,
    inspector: String,
    reviewer: String,
    previous_item_id: String,
    reason: String,
}

pub(super) async fn reinspect_route(
    State(state): State<AppState>,
    Path((tenant_id, freeze_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InspectionReinspectionRequest>,
) -> Response {
    let tenant = match quality_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_quality(&ctx, "quality.write") {
        return response.into_response();
    }
    let freeze = match uuid::Uuid::parse_str(freeze_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad freeze_id").into_response(),
    };
    let previous = match uuid::Uuid::parse_str(req.previous_item_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad previous_item_id").into_response(),
    };
    let request = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(response) => return response,
    };
    let reading = match req
        .reading_qty
        .as_deref()
        .map(erp_domain::quantity::parse_qty)
        .transpose()
    {
        Ok(value) => value.map(|q| q.as_micros()),
        Err(_) => return bad_request("bad reading_qty").into_response(),
    };
    let payload = match serde_json::to_string(&(
        2,
        "REINSPECTION",
        freeze,
        req.item_code.trim(),
        reading,
        req.inspector.trim(),
        req.reviewer.trim(),
        previous,
        req.reason.trim(),
    )) {
        Ok(value) => value,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "inspection payload unavailable",
            )
                .into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_inspection_outcome(
            tenant,
            &request,
            &payload,
            InspectionReadingInput {
                freeze_id: freeze,
                item_code: req.item_code.trim(),
                reading,
                inspector: req.inspector.trim(),
                declared_reviewer: req.reviewer.trim(),
                previous_item_id: Some(previous),
                reason: req.reason.trim(),
            },
            actor(&ctx),
        )
        .await
    {
        Ok(Ok(result)) => outcome_response(&request, result, "reading"),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => inspection_write_error(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InspectionReadingReviewRequest {
    request_id: String,
    decision: String,
    reason: String,
}

pub(super) async fn review_route(
    State(state): State<AppState>,
    Path((tenant_id, freeze_id, item_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InspectionReadingReviewRequest>,
) -> Response {
    let tenant = match quality_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_quality(&ctx, "quality.write") {
        return response.into_response();
    }
    let freeze = match uuid::Uuid::parse_str(freeze_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad freeze_id").into_response(),
    };
    let item = match uuid::Uuid::parse_str(item_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad item_id").into_response(),
    };
    let request = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(response) => return response,
    };
    let decision = req.decision.trim().to_ascii_uppercase();
    if !matches!(decision.as_str(), "APPROVE" | "REJECT") {
        return bad_request("bad review decision").into_response();
    }
    let payload = match serde_json::to_string(&(
        2,
        "READING_REVIEW",
        freeze,
        item,
        &decision,
        req.reason.trim(),
    )) {
        Ok(value) => value,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "inspection payload unavailable",
            )
                .into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .review_inspection_outcome(
            tenant,
            &request,
            &payload,
            InspectionReviewInput {
                freeze_id: freeze,
                item_id: item,
                decision: &decision,
                reason: req.reason.trim(),
            },
            actor(&ctx),
        )
        .await
    {
        Ok(Ok(result)) => outcome_response(&request, result, "review"),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => inspection_write_error(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InspectionReadingHistoryQuery {
    item_code: String,
}

pub(super) async fn history_route(
    State(state): State<AppState>,
    Path((tenant_id, freeze_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<InspectionReadingHistoryQuery>,
) -> Response {
    let tenant = match quality_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_quality(&ctx, "quality.read") {
        return response.into_response();
    }
    let freeze = match uuid::Uuid::parse_str(freeze_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad freeze_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_inspection_history(tenant, freeze, query.item_code.trim(), actor(&ctx))
        .await
    {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(error) => inspection_write_error(error),
    }
}
