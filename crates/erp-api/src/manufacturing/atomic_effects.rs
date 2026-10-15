//! Owned precise material issue and immutable original HTTP response.
use super::*;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AtomicEffectRequest {
    request_id: String,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    source_action: String,
    action_no: String,
    qty: String,
    amount_cent: i64,
    prepared_lines: i64,
    failing_line: bool,
}

pub(crate) async fn commit_atomic_effect_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Extension(rctx): Extension<RequestContext>,
    Json(req): Json<AtomicEffectRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.failing_line || req.prepared_lines != 1 {
        return bad_request("bad atomic compatibility preflight").into_response();
    }
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad atomic qty").into_response(),
    };
    let effect = erp_domain::manufacturing::AtomicEffect {
        source_action: req.source_action.trim().to_string(),
        action_no: req.action_no.trim().to_string(),
        qty,
        amount_cent: req.amount_cent,
    };
    let payload = match serde_json::to_string(&(
        2,
        tenant_id,
        ctx.tenant_id,
        ctx.user_id,
        work_order_id,
        req.warehouse_id,
        req.sku_id,
        effect.source_action.as_str(),
        effect.action_no.as_str(),
        effect.qty.as_micros(),
        effect.amount_cent,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let actor = ctx.production_release_actor(&rctx);
    match SharedContractRepo::new(state.db.pool()).commit_atomic_effect_with_actor(
        tenant_id,&request_id,&payload,
        erp_repo::shared_contract::AtomicEffectInput {work_order_id,warehouse_id:req.warehouse_id,sku_id:req.sku_id,effect:&effect},
        &actor).await {
        Ok(Ok(write)) => (StatusCode::OK,Json(serde_json::json!({
            "request_id":request_id,"status":"SUCCEEDED","replay":matches!(write.effect,SharedWrite::Replay(_)),
            "effect_id":write.document_id,"partial_success":false,"applied_lines":1,"snapshot":write.snapshot,
        }))).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => {
            if error.downcast_ref::<sqlx::Error>().is_some() {
                return (StatusCode::INTERNAL_SERVER_ERROR,"atomic material commit failed").into_response();
            }
            if matches!(error.downcast_ref::<erp_repo::shared_contract::ProductionReleaseError>(),
                Some(erp_repo::shared_contract::ProductionReleaseError::Forbidden)) {
                return forbidden("atomic material warehouse access denied").into_response();
            }
            match error.downcast_ref::<erp_repo::shared_contract::AtomicEffectError>() {
                Some(erp_repo::shared_contract::AtomicEffectError::NotFound) => not_found("atomic material source not found").into_response(),
                Some(erp_repo::shared_contract::AtomicEffectError::ReconciliationRequired) =>
                    (StatusCode::CONFLICT,Json(serde_json::json!({"code":"atomic_material_reconciliation_required"}))).into_response(),
                None => (StatusCode::UNPROCESSABLE_ENTITY,format!("{error}")).into_response(),
            }
        }
    }
}
