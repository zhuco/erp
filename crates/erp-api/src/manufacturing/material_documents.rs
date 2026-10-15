//! Versioned material drafts. No handler here confirms a stock or value effect.
use super::*;
use erp_repo::shared_contract::{
    MaterialDocumentError, MaterialDocumentLine, MaterialDocumentSave, ReceiptValueActor,
    ReceiptValueAuthorizationError,
};

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SaveRequest {
    request_id: String,
    document_id: uuid::Uuid,
    expected_version: i32,
    lines: Vec<LineRequest>,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct LineRequest {
    line_id: uuid::Uuid,
    kind: String,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    batch_id: Option<uuid::Uuid>,
    original_issue_id: Option<uuid::Uuid>,
    unit_code: String,
    qty: String,
    reason: Option<String>,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RejectionMode {
    #[serde(default)]
    rejection_envelope: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReadQuery {
    request_id: Option<String>,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceAdoptRequest {
    request_id: String,
    expected_version: i32,
    review_token: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ListQuery {
    offset: i64,
    limit: i64,
    status: Option<String>,
}

pub(super) async fn review_source(
    State(state): State<AppState>,
    Path((path, order, document)): Path<(String, uuid::Uuid, uuid::Uuid)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let actor = match actor(&ctx, epoch) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    match SharedContractRepo::new(state.db.pool())
        .review_material_document_source(tenant, order, document, actor)
        .await
    {
        Ok(review) => (StatusCode::OK, Json(review)).into_response(),
        Err(error) => failure(error),
    }
}

pub(super) async fn adopt_source(
    State(state): State<AppState>,
    Path((path, order, document)): Path<(String, uuid::Uuid, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
    Query(mode): Query<RejectionMode>,
    Json(req): Json<SourceAdoptRequest>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let request = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(response) => return response,
    };
    let fields = match serde_json::to_value(&req) {
        Ok(fields) => fields,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "material request encoding failed",
            )
                .into_response();
        }
    };
    let actor = match actor(&ctx, epoch) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    match SharedContractRepo::new(state.db.pool()).adopt_material_document_source(tenant,&request,
        erp_repo::shared_contract::MaterialDocumentSourceAdoption{document_id:document,work_order_id:order,expected_version:req.expected_version,review_token:&req.review_token},actor).await {
        Ok(Ok(result))=>(StatusCode::OK,Json(serde_json::json!({"replay":matches!(result.effect,SharedWrite::Replay(_)),"snapshot":result.snapshot}))).into_response(),
        Ok(Err(_))=>conflict("material source adoption command conflict").into_response(),Err(error)=>write_failure(error,mode.rejection_envelope,&request,&fields,&serde_json::json!({"tenant_id":tenant,"actor_user_id":ctx.user_id,"work_order_id":order,"document_id":document})),
    }
}

pub(super) async fn list(
    State(state): State<AppState>,
    Path((path, order)): Path<(String, uuid::Uuid)>,
    Query(query): Query<ListQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let actor = match actor(&ctx, epoch) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    match SharedContractRepo::new(state.db.pool()).list_material_documents(tenant,order,query.offset,query.limit,query.status.as_deref(),actor).await {
        Ok((total,data))=>(StatusCode::OK,Json(serde_json::json!({"total":total,"data":data,"offset":query.offset,"limit":query.limit}))).into_response(),
        Err(error)=>failure(error),
    }
}

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConfirmRequest {
    request_id: String,
    expected_version: i32,
}

pub(super) async fn confirm(
    State(state): State<AppState>,
    Path((path, order, document)): Path<(String, uuid::Uuid, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
    Query(mode): Query<RejectionMode>,
    Json(req): Json<ConfirmRequest>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let request = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(response) => return response,
    };
    let fields = match serde_json::to_value(&req) {
        Ok(fields) => fields,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "material request encoding failed",
            )
                .into_response();
        }
    };
    let actor = match actor(&ctx, epoch) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    match SharedContractRepo::new(state.db.pool()).confirm_material_document(tenant,&request,
        erp_repo::shared_contract::MaterialDocumentConfirm{document_id:document,work_order_id:order,expected_version:req.expected_version},actor).await {
        Ok(Ok(result))=>(StatusCode::OK,Json(serde_json::json!({"replay":matches!(result.effect,SharedWrite::Replay(_)),"snapshot":result.snapshot}))).into_response(),
        Ok(Err(_))=>conflict("material confirmation command conflict").into_response(),Err(error)=>write_failure(error,mode.rejection_envelope,&request,&fields,&serde_json::json!({"tenant_id":tenant,"actor_user_id":ctx.user_id,"work_order_id":order,"document_id":document})),
    }
}

fn actor(ctx: &AuthContext, epoch: VerifiedCredentialEpoch) -> Result<ReceiptValueActor, Response> {
    let (Some(session), Some(epoch)) = (ctx.session_id, epoch.0) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "current material session required",
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

pub(super) async fn save(
    State(state): State<AppState>,
    Path((path, order)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
    Query(mode): Query<RejectionMode>,
    Json(req): Json<SaveRequest>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let request = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(response) => return response,
    };
    let fields = match serde_json::to_value(&req) {
        Ok(fields) => fields,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "material request encoding failed",
            )
                .into_response();
        }
    };
    let actor = match actor(&ctx, epoch) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    if req.lines.is_empty() || req.lines.len() > 100 {
        return bad_request("material document requires 1 to 100 lines").into_response();
    }
    let mut lines = Vec::with_capacity(req.lines.len());
    for line in req.lines {
        if line.qty.len() > 64 {
            return bad_request("material quantity byte budget exceeded").into_response();
        }
        let quantity = match erp_domain::quantity::parse_qty(&line.qty) {
            Ok(qty) => qty,
            Err(_) => return bad_request("invalid material quantity").into_response(),
        };
        lines.push(MaterialDocumentLine {
            line_id: line.line_id,
            kind: line.kind,
            warehouse_id: line.warehouse_id,
            sku_id: line.sku_id,
            batch_id: line.batch_id,
            original_issue_id: line.original_issue_id,
            unit_code: line.unit_code,
            qty_micros: quantity.as_micros().to_string(),
            reason: line.reason,
        });
    }
    match SharedContractRepo::new(state.db.pool()).save_material_document(tenant,&request,MaterialDocumentSave{
        document_id:req.document_id,work_order_id:order,expected_version:req.expected_version,lines:&lines},actor).await {
        Ok(Ok(result))=>(StatusCode::OK,Json(serde_json::json!({"replay":matches!(result.effect,SharedWrite::Replay(_)),"snapshot":result.snapshot}))).into_response(),
        Ok(Err(_))=>conflict("material command payload conflict").into_response(),
        Err(error)=>write_failure(error,mode.rejection_envelope,&request,&fields,&serde_json::json!({"tenant_id":tenant,"actor_user_id":ctx.user_id,"work_order_id":order,"document_id":req.document_id})),
    }
}

pub(super) async fn read(
    State(state): State<AppState>,
    Path((path, order, document)): Path<(String, uuid::Uuid, uuid::Uuid)>,
    Query(query): Query<ReadQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let actor = match actor(&ctx, epoch) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    if query.request_id.as_ref().is_some_and(|id| {
        id.trim().is_empty() || id.len() > 128 || id.chars().any(char::is_control)
    }) {
        return bad_request("invalid original material request_id").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_material_document(tenant, order, document, query.request_id.as_deref(), actor)
        .await
    {
        Ok(snapshot) => (
            StatusCode::OK,
            Json(serde_json::json!({"snapshot":snapshot})),
        )
            .into_response(),
        Err(error) => failure(error),
    }
}

fn write_failure(
    error: anyhow::Error,
    envelope: bool,
    request: &str,
    fields: &serde_json::Value,
    context: &serde_json::Value,
) -> Response {
    match error.downcast::<erp_repo::shared_contract::MaterialDocumentRejected>() {
        Ok(rejected) => {
            if envelope {
                (StatusCode::OK,Json(serde_json::json!({"code":"material_document_rejected_before_commit","committed":false,
                    "request_id":request,"fields":fields,"context":context,"message":rejected.0.to_string()}))).into_response()
            } else {
                failure(rejected.0)
            }
        }
        Err(error) => failure(error),
    }
}

fn failure(error: anyhow::Error) -> Response {
    if let Some(auth) = error.downcast_ref::<ReceiptValueAuthorizationError>() {
        let status = match auth {
            ReceiptValueAuthorizationError::Unauthorized => StatusCode::UNAUTHORIZED,
            ReceiptValueAuthorizationError::Forbidden => StatusCode::FORBIDDEN,
            ReceiptValueAuthorizationError::Unavailable => StatusCode::NOT_FOUND,
        };
        return (status, "current material authority required").into_response();
    }
    match error.downcast_ref::<MaterialDocumentError>() {
        Some(MaterialDocumentError::VersionConflict) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"code":"VERSION_CONFLICT"})),
        )
            .into_response(),
        Some(MaterialDocumentError::SourceChanged) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"code":"MATERIAL_SOURCE_CHANGED"})),
        )
            .into_response(),
        Some(MaterialDocumentError::NotFound) => {
            not_found("material document source not found").into_response()
        }
        Some(MaterialDocumentError::ReconciliationRequired) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"code":"MATERIAL_DOCUMENT_RECONCILIATION_REQUIRED"})),
        )
            .into_response(),
        None => {
            if error.downcast_ref::<sqlx::Error>().is_some() {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "material document operation failed",
                )
                    .into_response()
            } else {
                (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()).into_response()
            }
        }
    }
}
