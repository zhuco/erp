use super::*;

use erp_repo::inventory_grid_report::{self, Error, InventoryAggregateQuery};

pub(crate) async fn post_inventory_aggregate(
    State(state): State<AppState>,
    Path(tenant_id): Path<TenantId>,
    Extension(ctx): Extension<AuthContext>,
    Json(query): Json<InventoryAggregateQuery>,
) -> Response {
    if uuid::Uuid::from(tenant_id).is_nil() {
        return bad_request("tenant_id must not be nil").into_response();
    }
    if let Err(response) = ensure_tenant_access(&ctx, tenant_id) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "OPS.REPORT.READ") {
        return response.into_response();
    }
    if let Err(error) = inventory_grid_report::validate(&query) {
        return inventory_aggregate_error(error);
    }
    if let Some(warehouse_id) = query.warehouse_id
        && let Err(response) = ensure_warehouse_access(&ctx, warehouse_id)
    {
        return response.into_response();
    }

    let warehouse_ids = warehouse_scope_filter(&ctx);
    match inventory_grid_report::aggregate(
        state.db.pool(),
        uuid::Uuid::from(tenant_id),
        &query,
        warehouse_ids.as_deref(),
    )
    .await
    {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(error) => inventory_aggregate_error(error),
    }
}

fn inventory_aggregate_error(error: Error) -> Response {
    match error {
        Error::Invalid(message) => bad_request(&message).into_response(),
        Error::MissingCost(missing_count) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "code": "inventory_cost_missing",
                "message": "current inventory cost is missing",
                "missing_count": missing_count,
            })),
        )
            .into_response(),
        Error::Storage(error) => {
            tracing::error!(%error, "inventory aggregate storage failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Inventory aggregate storage failed",
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState::new(
            Db::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
                .expect("lazy database URL is valid"),
            Arc::new(JwtService::new("test-secret".to_string())),
        )
    }

    fn context(tenant_id: TenantId) -> AuthContext {
        AuthContext {
            tenant_id,
            user_id: UserId(uuid::Uuid::from_u128(7)),
            perms: normalize_permissions(vec!["reports.read".to_string()]),
            is_admin: false,
            shop_ids: Vec::new(),
            warehouse_ids: vec![uuid::Uuid::from_u128(8)],
            merchant_ids: Vec::new(),
            organization_ids: Vec::new(),
        }
    }

    fn query(warehouse_id: Option<uuid::Uuid>) -> InventoryAggregateQuery {
        serde_json::from_value(serde_json::json!({
            "schema": 1,
            "warehouse_id": warehouse_id,
            "search": null,
            "by_category": false,
            "measure": "quantity"
        }))
        .expect("valid query")
    }

    #[tokio::test]
    async fn aggregate_rejects_auth_tenant_warehouse_and_query_before_database() {
        let tenant = TenantId(uuid::Uuid::from_u128(1));
        let other = TenantId(uuid::Uuid::from_u128(2));
        let nil = post_inventory_aggregate(
            State(state()),
            Path(TenantId(uuid::Uuid::nil())),
            Extension(context(tenant)),
            Json(query(None)),
        )
        .await;
        assert_eq!(nil.status(), StatusCode::BAD_REQUEST);

        let mismatch = post_inventory_aggregate(
            State(state()),
            Path(other),
            Extension(context(tenant)),
            Json(query(None)),
        )
        .await;
        assert_eq!(mismatch.status(), StatusCode::FORBIDDEN);

        let no_permission = post_inventory_aggregate(
            State(state()),
            Path(tenant),
            Extension(AuthContext {
                perms: Vec::new(),
                ..context(tenant)
            }),
            Json(query(None)),
        )
        .await;
        assert_eq!(no_permission.status(), StatusCode::FORBIDDEN);

        let invalid_query: InventoryAggregateQuery = serde_json::from_value(serde_json::json!({
            "schema": 2, "warehouse_id": null, "search": null,
            "by_category": false, "measure": "quantity"
        }))
        .expect("shape is deserializable");
        let invalid = post_inventory_aggregate(
            State(state()),
            Path(tenant),
            Extension(context(tenant)),
            Json(invalid_query),
        )
        .await;
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

        let denied = post_inventory_aggregate(
            State(state()),
            Path(tenant),
            Extension(context(tenant)),
            Json(query(Some(uuid::Uuid::from_u128(9)))),
        )
        .await;
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn read_only_allowance_is_exact_post_with_non_nil_tenant() {
        let tenant = uuid::Uuid::from_u128(1);
        let exact = format!("/v1/tenants/{tenant}/reports/inventory/aggregate");
        assert!(is_inventory_aggregate_read(&Method::POST, &exact));
        assert!(!is_inventory_aggregate_read(&Method::GET, &exact));
        assert!(!is_inventory_aggregate_read(
            &Method::POST,
            &format!("{exact}/extra")
        ));
        assert!(!is_inventory_aggregate_read(
            &Method::POST,
            "/v1/tenants/not-a-uuid/reports/inventory/aggregate"
        ));
        assert!(!is_inventory_aggregate_read(
            &Method::POST,
            "/v1/tenants/00000000-0000-0000-0000-000000000000/reports/inventory/aggregate"
        ));
    }
}
