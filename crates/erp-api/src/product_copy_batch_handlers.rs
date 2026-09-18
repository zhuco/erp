use super::*;

pub(crate) type CopyBatchStartRequest = erp_repo::product_copy_batches::StartRequest;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CopyBatchAdvanceRequest {
    pub max_items: u16,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CopyBatchRetryReviewRequest {
    pub request_id: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CopyBatchResultsQuery {
    #[serde(default)]
    pub after: Option<u64>,
    #[serde(default = "default_results_limit")]
    pub limit: u16,
    #[serde(default)]
    pub failed_only: bool,
}

fn default_results_limit() -> u16 {
    100
}

fn validate_start(
    request: &CopyBatchStartRequest,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if request.batch_id.is_nil() || request.token.is_nil() {
        return Err(bad_request("batch_id and token must be non-nil"));
    }
    if request.excluded.len() > 10_000
        || request.excluded.iter().any(uuid::Uuid::is_nil)
        || request
            .excluded
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != request.excluded.len()
    {
        return Err(bad_request(
            "excluded must contain unique non-nil product IDs",
        ));
    }
    if !(1..=100_000).contains(&request.confirmed_count) {
        return Err(bad_request("confirmed_count must be between 1 and 100000"));
    }
    Ok(())
}

fn batch_id(raw: &str) -> Result<uuid::Uuid, Response> {
    uuid::Uuid::parse_str(raw)
        .ok()
        .filter(|id| !id.is_nil())
        .ok_or_else(|| bad_request("bad batch_id").into_response())
}

pub(crate) async fn list_product_copy_batches(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> Response {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(r) = ensure_tenant_access(&ctx, tenant) {
        return r.into_response();
    }
    if let Err(r) = ensure_permission(&ctx, "skus.read") {
        return r.into_response();
    }
    match state
        .products_repo
        .list_copy_batches(tenant, ctx.user_id)
        .await
    {
        Ok(statuses) => (StatusCode::OK, Json(statuses)).into_response(),
        Err(error) => copy_batch_error(error),
    }
}

pub(crate) async fn start_product_copy_batch(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(request): Json<CopyBatchStartRequest>,
) -> Response {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(r) = ensure_tenant_access(&ctx, tenant) {
        return r.into_response();
    }
    for permission in ["skus.read", "skus.write"] {
        if let Err(r) = ensure_permission(&ctx, permission) {
            return r.into_response();
        }
    }
    if let Err(r) = validate_start(&request) {
        return r.into_response();
    }
    match state
        .products_repo
        .start_copy_batch(tenant, ctx.user_id, &request)
        .await
    {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(error) => copy_batch_error(error),
    }
}

pub(crate) async fn product_copy_batch_status(
    State(state): State<AppState>,
    Path((tenant_id, raw_batch)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> Response {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let batch = match batch_id(&raw_batch) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = ensure_tenant_access(&ctx, tenant) {
        return r.into_response();
    }
    if let Err(r) = ensure_permission(&ctx, "skus.read") {
        return r.into_response();
    }
    match state
        .products_repo
        .copy_batch_status(tenant, ctx.user_id, batch)
        .await
    {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(error) => copy_batch_error(error),
    }
}

pub(crate) async fn product_copy_batch_results(
    State(state): State<AppState>,
    Path((tenant_id, raw_batch)): Path<(String, String)>,
    Query(query): Query<CopyBatchResultsQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> Response {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let batch = match batch_id(&raw_batch) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = ensure_tenant_access(&ctx, tenant) {
        return r.into_response();
    }
    if let Err(r) = ensure_permission(&ctx, "skus.read") {
        return r.into_response();
    }
    if !(1..=100).contains(&query.limit) || query.after.is_some_and(|after| after >= 100_000) {
        return bad_request("invalid result page").into_response();
    }
    match state
        .products_repo
        .copy_batch_results(
            tenant,
            ctx.user_id,
            batch,
            query.after,
            query.limit,
            query.failed_only,
        )
        .await
    {
        Ok(results) => (StatusCode::OK, Json(results)).into_response(),
        Err(error) => copy_batch_error(error),
    }
}

pub(crate) async fn advance_product_copy_batch(
    State(state): State<AppState>,
    Path((tenant_id, raw_batch)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(request): Json<CopyBatchAdvanceRequest>,
) -> Response {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let batch = match batch_id(&raw_batch) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = ensure_tenant_access(&ctx, tenant) {
        return r.into_response();
    }
    for permission in ["skus.read", "skus.write"] {
        if let Err(r) = ensure_permission(&ctx, permission) {
            return r.into_response();
        }
    }
    if !(1..=25).contains(&request.max_items) {
        return bad_request("max_items must be between 1 and 25").into_response();
    }
    match state
        .products_repo
        .advance_copy_batch(tenant, ctx.user_id, batch, request.max_items)
        .await
    {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(error) => copy_batch_error(error),
    }
}

pub(crate) async fn cancel_product_copy_batch(
    State(state): State<AppState>,
    Path((tenant_id, raw_batch)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> Response {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let batch = match batch_id(&raw_batch) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = ensure_tenant_access(&ctx, tenant) {
        return r.into_response();
    }
    for permission in ["skus.read", "skus.write"] {
        if let Err(r) = ensure_permission(&ctx, permission) {
            return r.into_response();
        }
    }
    match state
        .products_repo
        .cancel_copy_batch(tenant, ctx.user_id, batch)
        .await
    {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(error) => copy_batch_error(error),
    }
}

pub(crate) async fn retry_product_copy_batch_review(
    State(state): State<AppState>,
    Path((tenant_id, raw_batch)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(request): Json<CopyBatchRetryReviewRequest>,
) -> Response {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let batch = match batch_id(&raw_batch) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if request.request_id.is_nil() {
        return bad_request("request_id must be non-nil").into_response();
    }
    if let Err(r) = ensure_tenant_access(&ctx, tenant) {
        return r.into_response();
    }
    for permission in ["skus.read", "skus.write"] {
        if let Err(r) = ensure_permission(&ctx, permission) {
            return r.into_response();
        }
    }
    match state
        .products_repo
        .retry_copy_batch_review(tenant, ctx.user_id, batch, request.request_id)
        .await
    {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(error) => copy_batch_retry_error(error),
    }
}

fn copy_batch_error(error: erp_repo::product_copy_batches::BatchError) -> Response {
    use erp_repo::product_copy_batches::BatchError;
    match error {
        BatchError::Invalid(message) => bad_request(&message).into_response(),
        BatchError::Unavailable => not_found("Copy batch unavailable").into_response(),
        BatchError::IdentityConflict | BatchError::Capacity => {
            conflict("Copy batch conflict").into_response()
        }
        BatchError::Storage(_) | BatchError::Serialization(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Copy batch storage failed",
        )
            .into_response(),
        BatchError::Action(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Copy batch outcome unknown",
        )
            .into_response(),
    }
}

pub(crate) fn copy_batch_retry_error(
    error: erp_repo::product_copy_batch_retries::RetryError,
) -> Response {
    use erp_repo::product_copy_batch_retries::RetryError;
    match error {
        RetryError::Invalid(_) => bad_request("Invalid retry-review request").into_response(),
        RetryError::Unavailable => not_found("Copy batch unavailable").into_response(),
        RetryError::IdentityConflict => conflict("Retry request identity conflict").into_response(),
        RetryError::NotTerminal => {
            conflict("Original batch still has pending or unknown items").into_response()
        }
        RetryError::NoRejected => conflict("Original batch has no rejected items").into_response(),
        RetryError::SourceUnavailable(source) => {
            conflict(&format!("Source product unavailable: {source}")).into_response()
        }
        RetryError::Capacity => conflict("Selection capacity exceeded").into_response(),
        RetryError::Storage(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Retry-review storage failed",
        )
            .into_response(),
    }
}

#[cfg(test)]
mod retry_error_tests {
    use super::*;
    use http_body_util::BodyExt;

    async fn body(response: Response) -> String {
        String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn retry_errors_are_distinguishable_and_storage_is_sanitized() {
        let source = uuid::Uuid::from_u128(91);
        for (error, expected) in [
            (
                erp_repo::product_copy_batch_retries::RetryError::NotTerminal,
                "pending or unknown",
            ),
            (
                erp_repo::product_copy_batch_retries::RetryError::NoRejected,
                "no rejected",
            ),
            (
                erp_repo::product_copy_batch_retries::RetryError::SourceUnavailable(source),
                "Source product unavailable",
            ),
        ] {
            let response = copy_batch_retry_error(error);
            assert_eq!(response.status(), StatusCode::CONFLICT);
            assert!(body(response).await.contains(expected));
        }
        let response =
            copy_batch_retry_error(erp_repo::product_copy_batch_retries::RetryError::Storage(
                sqlx::Error::Protocol("secret sql detail".into()),
            ));
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let message = body(response).await;
        assert!(!message.contains("secret sql detail"));
    }
}
