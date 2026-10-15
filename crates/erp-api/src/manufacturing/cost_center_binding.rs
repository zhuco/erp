use super::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    center_id: uuid::Uuid,
    reason: String,
}
pub(super) async fn post(
    State(state): State<AppState>,
    Path((path, order)): Path<(String, uuid::Uuid)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
    Json(input): Json<Input>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(t) => t,
        Err(response) => return response,
    };
    let (Some(session_id), Some(credential_epoch)) = (ctx.session_id, epoch.0) else {
        return (
            StatusCode::UNAUTHORIZED,
            "current cost binding session required",
        )
            .into_response();
    };
    let actor = erp_repo::shared_contract::CostValueActor {
        authentication_tenant: tenant,
        user_id: ctx.user_id,
        session_id,
        credential_epoch,
    };
    match SharedContractRepo::new(state.db.pool())
        .bind_cost_center(tenant, order, input.center_id, &input.reason, actor)
        .await
    {
        Ok(snapshot) => (StatusCode::OK, Json(snapshot)).into_response(),
        Err(error) => cost_response::error(error),
    }
}
