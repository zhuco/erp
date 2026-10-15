use super::*;
use erp_repo::shared_contract::{
    DispositionError, ReceiptDispositionInput, ReceiptValueActor, ReceiptValueAuthorizationError,
};

#[derive(Deserialize)]
pub(super) struct StateQuery {
    warehouse_id: uuid::Uuid,
}
pub(super) async fn read(
    State(state): State<AppState>,
    Path((path, receipt)): Path<(String, uuid::Uuid)>,
    Query(query): Query<StateQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let tenant = match TenantId::from_str(&path) {
        Ok(t) => t,
        Err(_) => return bad_request("bad tenant").into_response(),
    };
    if tenant != ctx.tenant_id {
        return forbidden("business tenant mismatch").into_response();
    }
    let (Some(session), Some(epoch)) = (ctx.session_id, epoch.0) else {
        return (StatusCode::UNAUTHORIZED, "current session required").into_response();
    };
    let actor = ReceiptValueActor {
        authentication_tenant: tenant,
        user_id: ctx.user_id,
        session_id: session,
        credential_epoch: epoch,
        warehouse_id: query.warehouse_id,
    };
    match SharedContractRepo::new(state.db.pool())
        .receipt_disposition_state(tenant, receipt, actor)
        .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(error) => {
            let status = match error.downcast_ref::<ReceiptValueAuthorizationError>() {
                Some(ReceiptValueAuthorizationError::Unauthorized) => StatusCode::UNAUTHORIZED,
                Some(ReceiptValueAuthorizationError::Forbidden) => StatusCode::FORBIDDEN,
                Some(ReceiptValueAuthorizationError::Unavailable) => StatusCode::NOT_FOUND,
                _ => {
                    if error.downcast_ref::<sqlx::Error>().is_some() {
                        StatusCode::INTERNAL_SERVER_ERROR
                    } else {
                        StatusCode::CONFLICT
                    }
                }
            };
            (
                status,
                if status == StatusCode::INTERNAL_SERVER_ERROR {
                    "quality storage unavailable".into()
                } else {
                    error.to_string()
                },
            )
                .into_response()
        }
    }
}

pub(super) async fn post(
    state: &AppState,
    ctx: &AuthContext,
    epoch: Option<i32>,
    tenant: TenantId,
    request: &str,
    req: &QualityDispositionRequest,
) -> Response {
    if req.receipt_id.is_some() && req.hold_id.is_some() {
        return bad_request("choose the original receipt, not both source protocols")
            .into_response();
    }
    let Some(receipt) = req.receipt_id else {
        return (StatusCode::CONFLICT,Json(serde_json::json!({"code":"quality_source_reconcile_required",
            "message":"quality disposition needs an original production receipt, not an unproven hold counter",
            "reconciliation_required":true}))).into_response();
    };
    let (Some(session), Some(epoch)) = (ctx.session_id, epoch) else {
        return (
            StatusCode::UNAUTHORIZED,
            "current business session required",
        )
            .into_response();
    };
    let location = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad location_id").into_response(),
    };
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad qty").into_response(),
    };
    let rework = if req.rework_work_order_id.trim().is_empty() {
        None
    } else {
        match uuid::Uuid::parse_str(req.rework_work_order_id.trim()) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad rework order").into_response(),
        }
    };
    let input = ReceiptDispositionInput {
        receipt_id: receipt,
        parent_disposition_id: req.parent_disposition_id,
        returned_source_move_id: req.returned_source_move_id,
        warehouse_id: req.warehouse_id,
        location_id: location,
        sku_id: req.sku_id,
        kind: req.kind.trim().into(),
        qty_micros: qty.as_micros(),
        reason: req.reason.trim().into(),
        rework_work_order_id: rework,
    };
    let actor = ReceiptValueActor {
        authentication_tenant: ctx.tenant_id,
        user_id: ctx.user_id,
        session_id: session,
        credential_epoch: epoch,
        warehouse_id: req.warehouse_id,
    };
    match SharedContractRepo::new(state.db.pool()).dispose_receipt(tenant,request,&input,actor).await {
        Ok(Ok((write,result)))=>(StatusCode::OK,Json(serde_json::json!({"replay":matches!(write,SharedWrite::Replay(_)),"result":result}))).into_response(),
        Ok(Err(_))=>conflict("quality command intent conflict").into_response(),
        Err(error)=>{
            let status=match error.downcast_ref::<ReceiptValueAuthorizationError>() {
                Some(ReceiptValueAuthorizationError::Unauthorized)=>StatusCode::UNAUTHORIZED,
                Some(ReceiptValueAuthorizationError::Forbidden)=>StatusCode::FORBIDDEN,
                Some(ReceiptValueAuthorizationError::Unavailable)=>StatusCode::NOT_FOUND,
                _=>if error.downcast_ref::<sqlx::Error>().is_some() {StatusCode::INTERNAL_SERVER_ERROR}else{StatusCode::CONFLICT},
            };
            let reconcile=matches!(error.downcast_ref::<DispositionError>(),Some(DispositionError::Reconcile(_)))
                || matches!(error.downcast_ref::<erp_repo::shared_contract::ProductionReleaseError>(),Some(erp_repo::shared_contract::ProductionReleaseError::ReconciliationRequired(_)));
            (status,Json(serde_json::json!({"code":if reconcile{"quality_source_reconcile_required"}else{"quality_disposition_refused"},
                "message":if status==StatusCode::INTERNAL_SERVER_ERROR{"quality storage unavailable".into()}else{error.to_string()},
                "reconciliation_required":reconcile}))).into_response()
        }
    }
}
