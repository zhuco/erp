use super::*;

#[tokio::test]
async fn sku_find_rejects_invalid_parent_tenant_permission_and_request_before_database() {
    let tenant = TenantId(uuid::Uuid::from_u128(21));
    for (path_tenant, product, permissions, text, expected) in [
        (
            tenant,
            uuid::Uuid::nil(),
            vec!["skus.read".to_string()],
            "a",
            StatusCode::BAD_REQUEST,
        ),
        (
            TenantId(uuid::Uuid::from_u128(99)),
            uuid::Uuid::from_u128(22),
            vec!["skus.read".to_string()],
            "a",
            StatusCode::FORBIDDEN,
        ),
        (
            tenant,
            uuid::Uuid::from_u128(22),
            vec![],
            "a",
            StatusCode::FORBIDDEN,
        ),
        (
            tenant,
            uuid::Uuid::from_u128(22),
            vec!["skus.read".to_string()],
            "",
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let state = AppState::new(
            Db::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused").unwrap(),
            Arc::new(JwtService::new("fixture".into())),
        );
        let auth = AuthContext {
            tenant_id: tenant,
            user_id: UserId(uuid::Uuid::from_u128(23)),
            perms: normalize_permissions(permissions),
            is_admin: false,
            shop_ids: vec![],
            warehouse_ids: vec![],
            merchant_ids: vec![],
            organization_ids: vec![],
        };
        let request = erp_repo::product_grid_find::FindRequest {
            query: serde_json::from_value(
                serde_json::json!({"schema":1,"offset":0,"limit":100,"filters":{},"sorts":[]}),
            )
            .unwrap(),
            text: text.into(),
            columns: vec![erp_grid_core::ColumnId(8)],
            case_sensitive: false,
            backward: false,
            after: None,
        };
        let response = find_sku_cell(
            State(state),
            Path((path_tenant.to_string(), product.to_string())),
            Extension(auth),
            Json(request),
        )
        .await
        .into_response();
        assert_eq!(response.status(), expected);
    }
}
