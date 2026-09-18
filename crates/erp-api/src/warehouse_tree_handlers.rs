use super::*;
use erp_repo::warehouse_tree::{self, Error, TreeQuery};

pub(crate) async fn query_warehouse_tree(
    State(state): State<AppState>,
    Path(tenant): Path<TenantId>,
    Extension(ctx): Extension<AuthContext>,
    Json(query): Json<TreeQuery>,
) -> Response {
    if tenant.0.is_nil() {
        return bad_request("tenant_id must not be nil").into_response();
    }
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    for permission in ["warehouses.read", "inventory.read"] {
        if let Err(response) = ensure_permission(&ctx, permission) {
            return response.into_response();
        }
    }
    if let Err(error) = warehouse_tree::validate(&query) {
        return tree_error(error);
    }
    if let Some(parent) = query.parent
        && let Err(response) = ensure_warehouse_access(&ctx, parent)
    {
        return response.into_response();
    }
    let warehouses = warehouse_scope_filter(&ctx);
    match warehouse_tree::query(state.db.pool(), tenant.0, &query, warehouses.as_deref()).await {
        Ok(page) => Json(page).into_response(),
        Err(error) => tree_error(error),
    }
}

fn tree_error(error: Error) -> Response {
    match error {
        Error::Invalid(message) => bad_request(&message).into_response(),
        Error::NotFound => (StatusCode::NOT_FOUND, "Warehouse unavailable").into_response(),
        Error::Storage(error) => {
            tracing::error!(%error, "warehouse tree query failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Warehouse tree query failed",
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn warehouse_tree_denies_tenant_permissions_scope_and_invalid_query_before_database() {
        let tenant = TenantId(uuid::Uuid::from_u128(1));
        let allowed = uuid::Uuid::from_u128(2);
        for (path, permissions, parent, schema) in [
            (
                TenantId(uuid::Uuid::from_u128(99)),
                vec!["warehouses.read", "inventory.read"],
                None,
                1,
            ),
            (tenant, vec!["warehouses.read"], None, 1),
            (tenant, vec!["inventory.read"], None, 1),
            (
                tenant,
                vec!["warehouses.read", "inventory.read"],
                Some(uuid::Uuid::from_u128(99)),
                1,
            ),
            (
                tenant,
                vec!["warehouses.read", "inventory.read"],
                Some(allowed),
                2,
            ),
        ] {
            let state = AppState::new(
                Db::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused").unwrap(),
                Arc::new(JwtService::new("fixture".into())),
            );
            let context = AuthContext {
                tenant_id: tenant,
                user_id: UserId(uuid::Uuid::from_u128(3)),
                perms: normalize_permissions(permissions.into_iter().map(str::to_owned).collect()),
                is_admin: false,
                shop_ids: vec![],
                warehouse_ids: vec![allowed],
                merchant_ids: vec![],
                organization_ids: vec![],
            };
            let request = serde_json::from_value(serde_json::json!({
                "schema":schema,"parent":parent,"after":null,"limit":100,"search":null
            }))
            .unwrap();
            let response =
                query_warehouse_tree(State(state), Path(path), Extension(context), Json(request))
                    .await;
            assert_eq!(
                response.status(),
                if schema == 2 {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::FORBIDDEN
                }
            );
        }
    }

    #[test]
    fn warehouse_tree_read_only_route_is_exact() {
        let path = format!("/v1/tenants/{}/warehouses/tree", uuid::Uuid::from_u128(1));
        assert!(is_warehouse_tree_read(&Method::POST, &path));
        assert!(!is_warehouse_tree_read(&Method::DELETE, &path));
        assert!(!is_warehouse_tree_read(
            &Method::POST,
            &format!("{path}/extra")
        ));
        assert!(!is_warehouse_tree_read(
            &Method::POST,
            "/v1/tenants/no/warehouses/tree"
        ));
        assert!(!is_warehouse_tree_read(
            &Method::POST,
            &format!("/v1/tenants/{}/warehouses/tree", uuid::Uuid::nil())
        ));
    }
}
