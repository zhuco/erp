//! Real selector queries with current stored manufacturing read authority.
use super::*;
use erp_repo::shared_contract::{
    MaterialDocumentError, ReceiptValueActor, ReceiptValueAuthorizationError, WorkOrderOptionQuery,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OptionsQuery {
    #[serde(default)]
    query: String,
    status: Option<String>,
    #[serde(default = "open_default")]
    open_only: bool,
    offset: i64,
    limit: i64,
}
fn open_default() -> bool {
    true
}
pub(super) fn actor(
    ctx: &AuthContext,
    epoch: VerifiedCredentialEpoch,
) -> Result<ReceiptValueActor, Response> {
    let (Some(session), Some(epoch)) = (ctx.session_id, epoch.0) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "current manufacturing session required",
        )
            .into_response());
    };
    Ok(ReceiptValueActor {
        authentication_tenant: ctx.tenant_id,
        user_id: ctx.user_id,
        session_id: session,
        credential_epoch: epoch,
        warehouse_id: uuid::Uuid::nil(),
    })
}
pub(super) async fn list(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<OptionsQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(t) => t,
        Err(response) => return response,
    };
    let actor = match actor(&ctx, epoch) {
        Ok(a) => a,
        Err(response) => return response,
    };
    match SharedContractRepo::new(state.db.pool()).list_work_order_options(tenant,WorkOrderOptionQuery{
        text:&query.query,status:query.status.as_deref(),open_only:query.open_only,offset:query.offset,limit:query.limit},actor).await {
        Ok((total,data))=>(StatusCode::OK,Json(serde_json::json!({"total":total,"data":data,"offset":query.offset,"limit":query.limit}))).into_response(),Err(error)=>failure(error),
    }
}
pub(super) async fn context(
    State(state): State<AppState>,
    Path((path, order)): Path<(String, uuid::Uuid)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(t) => t,
        Err(response) => return response,
    };
    let actor = match actor(&ctx, epoch) {
        Ok(a) => a,
        Err(response) => return response,
    };
    match SharedContractRepo::new(state.db.pool())
        .read_work_order_material_context(tenant, order, actor)
        .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(error) => failure(error),
    }
}
pub(super) fn failure(error: anyhow::Error) -> Response {
    if let Some(auth) = error.downcast_ref::<ReceiptValueAuthorizationError>() {
        let status = match auth {
            ReceiptValueAuthorizationError::Unauthorized => StatusCode::UNAUTHORIZED,
            ReceiptValueAuthorizationError::Forbidden => StatusCode::FORBIDDEN,
            ReceiptValueAuthorizationError::Unavailable => StatusCode::NOT_FOUND,
        };
        return (status, "current manufacturing read authority required").into_response();
    }
    if matches!(
        error.downcast_ref::<MaterialDocumentError>(),
        Some(MaterialDocumentError::NotFound)
    ) {
        return not_found("work order outside enterprise").into_response();
    }
    if error.downcast_ref::<sqlx::Error>().is_some() {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "work order query storage unavailable",
        )
            .into_response()
    } else {
        (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()).into_response()
    }
}
