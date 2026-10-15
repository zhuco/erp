use super::*;

pub(super) async fn read(
    State(state): State<AppState>,
    Path((path, order, run)): Path<(String, uuid::Uuid, uuid::Uuid)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let tenant = match TenantId::from_str(&path) {
        Ok(tenant) => tenant,
        Err(_) => return bad_request("bad tenant").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    let (Some(session_id), Some(credential_epoch)) = (ctx.session_id, epoch.0) else {
        return (
            StatusCode::UNAUTHORIZED,
            "current cost query session required",
        )
            .into_response();
    };
    let actor = erp_repo::shared_contract::CostValueActor {
        authentication_tenant: ctx.tenant_id,
        user_id: ctx.user_id,
        session_id,
        credential_epoch,
    };
    match SharedContractRepo::new(state.db.pool())
        .work_order_cost_evidence(tenant, order, run, actor)
        .await
    {
        Ok(evidence) => (StatusCode::OK, Json(evidence)).into_response(),
        Err(error) => cost_response::error(error),
    }
}
