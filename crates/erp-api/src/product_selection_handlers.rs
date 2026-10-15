use super::*;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SelectionPageRequest {
    pub offset: u64,
    pub limit: u16,
    #[serde(default)]
    pub sorts: Vec<erp_grid_core::SortRule>,
    #[serde(default)]
    pub query_order: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SelectionCountRequest {
    pub excluded: Vec<uuid::Uuid>,
}

pub(crate) async fn freeze_product_selection(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(query): Json<erp_grid_core::paged_query::PageQuery>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    if let Err(message) = erp_repo::product_grid::validate(&query) {
        return bad_request(message).into_response();
    }
    match state
        .products_repo
        .freeze_selection(tenant, ctx.user_id, &query)
        .await
    {
        Ok(token) => (StatusCode::OK, Json(token)).into_response(),
        Err(error) => product_selection_error(error),
    }
}

pub(crate) async fn product_selection_page(
    State(state): State<AppState>,
    Path((tenant_id, token)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(request): Json<SelectionPageRequest>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let token_uuid = match uuid::Uuid::parse_str(&token) {
        Ok(value) => value,
        Err(_) => return bad_request("bad selection token").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    if let Err(error) = erp_repo::product_selection::validate_sorts(&request.sorts) {
        return product_selection_error(error);
    }
    match state
        .products_repo
        .selection_page_query_order(
            tenant,
            ctx.user_id,
            token_uuid,
            request.offset,
            request.limit,
            &request.sorts,
            request.query_order,
        )
        .await
    {
        Ok(page) => (StatusCode::OK, Json(page)).into_response(),
        Err(error) => product_selection_error(error),
    }
}

pub(crate) async fn product_selection_count(
    State(state): State<AppState>,
    Path((tenant_id, token)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(request): Json<SelectionCountRequest>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let token_uuid = match uuid::Uuid::parse_str(&token) {
        Ok(value) => value,
        Err(_) => return bad_request("bad selection token").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    match state
        .products_repo
        .selection_count(tenant, ctx.user_id, token_uuid, &request.excluded)
        .await
    {
        Ok(affected) => (
            StatusCode::OK,
            Json(serde_json::json!({ "affected": affected })),
        )
            .into_response(),
        Err(error) => product_selection_error(error),
    }
}

fn product_selection_error(error: erp_repo::product_selection::SelectionError) -> Response {
    use erp_repo::product_selection::SelectionError;
    match error {
        SelectionError::Invalid(message) => bad_request(&message).into_response(),
        SelectionError::Unavailable => (StatusCode::GONE, "Selection unavailable").into_response(),
        SelectionError::Capacity => conflict("Selection capacity exceeded").into_response(),
        SelectionError::Storage(error) => {
            tracing::error!(%error, "product selection storage failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Selection storage failed",
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_bodies_reject_unknown_fields() {
        let request = serde_json::from_value::<SelectionPageRequest>(
            serde_json::json!({"offset": 0, "limit": 50}),
        )
        .unwrap();
        assert!(request.sorts.is_empty());
        assert!(!request.query_order);
        let query_order = serde_json::from_value::<SelectionPageRequest>(
            serde_json::json!({"offset": 0, "limit": 50, "query_order": true}),
        )
        .unwrap();
        assert!(query_order.query_order);
        assert!(
            serde_json::from_value::<SelectionPageRequest>(
                serde_json::json!({"offset": 0, "limit": 50, "extra": true})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<SelectionCountRequest>(
                serde_json::json!({"excluded": [], "extra": true})
            )
            .is_err()
        );
    }

    #[test]
    fn selection_routes_require_exact_valid_uuid_segments_for_read_only_bypass() {
        let method = axum::http::Method::POST;
        let tenant = uuid::Uuid::from_u128(1);
        let token = uuid::Uuid::from_u128(2);
        assert!(is_product_grid_read(
            &method,
            &format!("/v1/tenants/{tenant}/products/selections/{token}/page")
        ));
        assert!(is_product_grid_read(
            &method,
            &format!("/v1/tenants/{tenant}/products/selections/{token}/count")
        ));
        assert!(!is_product_grid_read(
            &method,
            &format!("/v1/tenants/{tenant}/products/selections/not-a-uuid/page")
        ));
        assert!(!is_product_grid_read(
            &axum::http::Method::GET,
            &format!("/v1/tenants/{tenant}/products/selections")
        ));
    }

    #[test]
    fn selection_auth_checks_tenant_and_permission() {
        let tenant = TenantId(uuid::Uuid::from_u128(1));
        let other = TenantId(uuid::Uuid::from_u128(2));
        let ctx = AuthContext {
            tenant_id: tenant,
            user_id: UserId(uuid::Uuid::from_u128(3)),
            perms: normalize_permissions(vec!["skus.read".into()]),
            is_admin: false,
            shop_ids: vec![],
            warehouse_ids: vec![],
            merchant_ids: vec![],
            organization_ids: vec![],
        session_id: None,
        };
        assert!(ensure_tenant_access(&ctx, tenant).is_ok());
        assert_eq!(
            ensure_tenant_access(&ctx, other).unwrap_err().0,
            StatusCode::FORBIDDEN
        );
        assert!(ensure_permission(&ctx, "skus.read").is_ok());
        let denied = AuthContext {
            perms: vec![],
            ..ctx
        };
        assert_eq!(
            ensure_permission(&denied, "skus.read").unwrap_err().0,
            StatusCode::FORBIDDEN
        );
    }
}

pub(crate) async fn find_product_selection_cell(
    State(state): State<AppState>,
    Path((tenant_id, token)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(request): Json<erp_repo::product_grid_find::FindRequest>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let token = match uuid::Uuid::parse_str(&token) {
        Ok(value) if !value.is_nil() => value,
        _ => return bad_request("bad selection token").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    if let Err(error) = erp_repo::product_selection_find::validate(&request) {
        return product_selection_error(error);
    }
    match erp_repo::product_selection_find::find(
        state.db.pool(),
        tenant,
        ctx.user_id,
        token,
        &request,
    )
    .await
    {
        Ok(hit) => (StatusCode::OK, Json(serde_json::json!({"hit":hit}))).into_response(),
        Err(error) => product_selection_error(error),
    }
}

#[cfg(test)]
mod frozen_find_tests {
    use super::*;
    #[tokio::test]
    async fn frozen_find_denies_wrong_tenant_and_missing_permission_before_query() {
        let tenant = TenantId(uuid::Uuid::from_u128(1));
        for (path_tenant, perms) in [
            (
                TenantId(uuid::Uuid::from_u128(99)),
                vec!["skus.read".to_string()],
            ),
            (tenant, vec![]),
        ] {
            let state = AppState::new(
                Db::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused").unwrap(),
                Arc::new(JwtService::new("fixture".into())),
            );
            let ctx = AuthContext {
                tenant_id: tenant,
                user_id: UserId(uuid::Uuid::from_u128(2)),
                perms: normalize_permissions(perms),
                is_admin: false,
                shop_ids: vec![],
                warehouse_ids: vec![],
                merchant_ids: vec![],
                organization_ids: vec![],
            session_id: None,
            };
            let request=serde_json::from_value(serde_json::json!({"query":{"schema":1,"offset":0,"limit":100,"filters":{},"sorts":[]},"text":"a","columns":[0]})).unwrap();
            let response = find_product_selection_cell(
                State(state),
                Path((
                    path_tenant.to_string(),
                    uuid::Uuid::from_u128(3).to_string(),
                )),
                Extension(ctx),
                Json(request),
            )
            .await
            .into_response();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }

    #[test]
    fn frozen_find_read_routes_are_exact_and_do_not_admit_writes() {
        let tenant = uuid::Uuid::from_u128(1);
        let token = uuid::Uuid::from_u128(2);
        for path in [
            format!("/v1/tenants/{tenant}/products/find"),
            format!("/v1/tenants/{tenant}/products/{token}/skus/find"),
            format!("/v1/tenants/{tenant}/products/selections/{token}/find"),
        ] {
            assert!(is_product_grid_read(&axum::http::Method::POST, &path));
            assert!(!is_product_grid_read(&axum::http::Method::DELETE, &path));
            assert!(!is_product_grid_read(
                &axum::http::Method::POST,
                &format!("{path}/action")
            ));
        }
        assert!(!is_product_grid_read(
            &axum::http::Method::POST,
            &format!("/v1/tenants/{tenant}/products/selections/not-a-uuid/find")
        ));
    }
}
