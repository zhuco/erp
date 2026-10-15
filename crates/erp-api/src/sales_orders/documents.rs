use super::*;
use erp_repo::shared_contract::{
    ReceiptValueActor, ReceiptValueAuthorizationError, SalesDocumentInput, SalesDocumentLine,
    SharedContractRepo,
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    request_id: String,
    so_id: Option<uuid::Uuid>,
    document_no: String,
    customer_code: String,
    currency: String,
    action: String,
    expected_version: Option<i32>,
    items: Vec<Line>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Line {
    line_no: i32,
    sku_code: String,
    qty: String,
    price_cent: i64,
    discount_cent: i64,
    tax_cent: i64,
    due_date: String,
    address: String,
    source_kind: String,
    warehouse_id: Option<uuid::Uuid>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReadQuery {
    request_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ListQuery {
    after_document_no: Option<String>,
    limit: Option<i64>,
}

fn identity(
    path: &str,
    ctx: &AuthContext,
    epoch: Option<i32>,
) -> Result<(TenantId, ReceiptValueActor), Response> {
    let tenant = TenantId::from_str(path).map_err(|_| bad_request("bad tenant").into_response())?;
    if tenant != ctx.tenant_id {
        return Err(forbidden("business tenant mismatch").into_response());
    }
    let (Some(session_id), Some(credential_epoch)) = (ctx.session_id, epoch) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "current business session required",
        )
            .into_response());
    };
    Ok((
        tenant,
        ReceiptValueActor {
            authentication_tenant: ctx.tenant_id,
            user_id: ctx.user_id,
            session_id,
            credential_epoch,
            warehouse_id: uuid::Uuid::nil(),
        },
    ))
}
fn error(error: anyhow::Error) -> Response {
    if let Some(auth) = error.downcast_ref::<ReceiptValueAuthorizationError>() {
        let status = match auth {
            ReceiptValueAuthorizationError::Unauthorized => StatusCode::UNAUTHORIZED,
            ReceiptValueAuthorizationError::Forbidden => StatusCode::FORBIDDEN,
            ReceiptValueAuthorizationError::Unavailable => StatusCode::NOT_FOUND,
        };
        return (
            status,
            "trade document is unavailable in the current authority scope",
        )
            .into_response();
    }
    if error.downcast_ref::<sqlx::Error>().is_some() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "trade document storage unavailable",
        )
            .into_response();
    }
    (StatusCode::CONFLICT, error.to_string()).into_response()
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct SaveQuery {
    #[serde(default)]
    rejection_envelope: bool,
}
pub(super) async fn save(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
    Query(query): Query<SaveQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
    Json(input): Json<Input>,
) -> Response {
    let (tenant, actor) = match identity(&path, &ctx, epoch.0) {
        Ok(identity) => identity,
        Err(error) => return error,
    };
    let request = match shared_request_id(&headers, Some(&input.request_id)) {
        Ok(Some(request)) => request,
        Ok(None) => return bad_request("original request identity required").into_response(),
        Err(response) => return response,
    };
    if input.items.is_empty() || input.items.len() > 1000 {
        return bad_request("trade line budget invalid").into_response();
    }
    let rejection_fields = if query.rejection_envelope {
        match serde_json::to_value(&input) {
            Ok(fields) => Some(fields),
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "trade request encoding unavailable",
                )
                    .into_response();
            }
        }
    } else {
        None
    };
    let mut lines = Vec::with_capacity(input.items.len());
    for item in &input.items {
        let quantity = match erp_domain::trade::document_qty_micros(&item.qty) {
            Ok(quantity) => quantity,
            Err(_) => return bad_request("invalid exact trade quantity").into_response(),
        };
        lines.push(SalesDocumentLine {
            warehouse_id: item.warehouse_id,
            line: erp_domain::trade::SalesLine {
                line_no: item.line_no,
                item_code: item.sku_code.trim().into(),
                qty_micros: quantity,
                price_cent: item.price_cent,
                discount_cent: item.discount_cent,
                tax_cent: item.tax_cent,
                due_date: item.due_date.trim().into(),
                address: item.address.trim().into(),
                source: item.source_kind.trim().into(),
            },
        });
    }
    let input = SalesDocumentInput {
        request_id: &request,
        order_id: input.so_id,
        document_no: input.document_no.trim(),
        customer_code: input.customer_code.trim(),
        currency: input.currency.trim(),
        action: input.action.trim(),
        expected_version: input.expected_version,
        lines: &lines,
    };
    match SharedContractRepo::new(state.db.pool())
        .save_sales_document(tenant, input, actor)
        .await
    {
        Ok(document) => (StatusCode::OK, Json(document)).into_response(),
        Err(e) => {
            if let (Some(fields), Some(rejected)) = (
                rejection_fields,
                e.downcast_ref::<erp_repo::shared_contract::SalesDocumentRejected>(),
            ) {
                return (StatusCode::OK,Json(serde_json::json!({"code":"trade_document_rejected_before_commit","committed":false,"request_id":request,"fields":fields,"message":rejected.0}))).into_response();
            }
            error(e)
        }
    }
}
pub(super) async fn read(
    State(state): State<AppState>,
    Path((path, order)): Path<(String, uuid::Uuid)>,
    Query(query): Query<ReadQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let (tenant, actor) = match identity(&path, &ctx, epoch.0) {
        Ok(identity) => identity,
        Err(error) => return error,
    };
    if query
        .request_id
        .as_ref()
        .is_some_and(|request| request.trim().is_empty() || request.len() > 128)
    {
        return bad_request("bad original request identity").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_sales_document(tenant, order, query.request_id.as_deref(), actor)
        .await
    {
        Ok(document) => (StatusCode::OK, Json(document)).into_response(),
        Err(e) => error(e),
    }
}
pub(super) async fn list(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<ListQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let (tenant, actor) = match identity(&path, &ctx, epoch.0) {
        Ok(identity) => identity,
        Err(error) => return error,
    };
    let limit = query.limit.unwrap_or(500);
    if ![100, 200, 300, 500, 800, 1000, 2000].contains(&limit)
        || query
            .after_document_no
            .as_ref()
            .is_some_and(|s| s.len() > 128)
    {
        return bad_request("bad trade page budget or cursor").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .list_sales_documents(tenant, query.after_document_no.as_deref(), limit, actor)
        .await
    {
        Ok(page) => (StatusCode::OK, Json(page)).into_response(),
        Err(e) => error(e),
    }
}

pub(super) async fn command(
    State(state): State<AppState>,
    Path((path, request)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let (tenant, actor) = match identity(&path, &ctx, epoch.0) {
        Ok(identity) => identity,
        Err(error) => return error,
    };
    match SharedContractRepo::new(state.db.pool())
        .read_sales_document_command(tenant, &request, actor)
        .await
    {
        Ok(document) => (StatusCode::OK, Json(document)).into_response(),
        Err(e) => error(e),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NumberQuery {
    document_no: String,
}
pub(super) async fn lookup(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<NumberQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let (tenant, actor) = match identity(&path, &ctx, epoch.0) {
        Ok(identity) => identity,
        Err(error) => return error,
    };
    match SharedContractRepo::new(state.db.pool())
        .read_sales_document_number(tenant, &query.document_no, actor)
        .await
    {
        Ok(document) => (StatusCode::OK, Json(document)).into_response(),
        Err(e) => error(e),
    }
}
