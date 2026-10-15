use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

#[derive(Default)]
struct Repo {
    calls: Mutex<Vec<(TenantId, uuid::Uuid)>>,
    range_calls: Mutex<
        Vec<(
            TenantId,
            uuid::Uuid,
            erp_repo::sku_grid::SkuGridRangesRequest,
        )>,
    >,
    action_calls: Mutex<Vec<(TenantId, UserId, erp_grid_core::actions::ActionCommand)>>,
    receipt_calls: Mutex<Vec<(TenantId, UserId, erp_grid_core::RowId)>>,
}

#[async_trait]
impl SkuRepo for Repo {
    async fn execute_grid_action(
        &self,
        tenant: TenantId,
        actor: UserId,
        command: &erp_grid_core::actions::ActionCommand,
    ) -> Result<erp_grid_core::actions::ActionReceipt, erp_repo::product_actions::ActionError> {
        self.action_calls
            .lock()
            .unwrap()
            .push((tenant, actor, command.clone()));
        Ok(erp_grid_core::actions::ActionReceipt {
            operation_id: command.operation_id,
            source: command.source,
            command_fingerprint: command.fingerprint(),
            outcome: erp_grid_core::actions::ActionOutcome::Rejected {
                code: "FIXTURE".into(),
                message: "fixture".into(),
            },
        })
    }
    async fn grid_action_receipt(
        &self,
        tenant: TenantId,
        actor: UserId,
        operation: erp_grid_core::RowId,
    ) -> Result<Option<erp_grid_core::actions::ActionReceipt>, erp_repo::product_actions::ActionError>
    {
        self.receipt_calls
            .lock()
            .unwrap()
            .push((tenant, actor, operation));
        Ok(None)
    }
    async fn query_skus(
        &self,
        tenant: TenantId,
        product: uuid::Uuid,
        _query: &erp_grid_core::paged_query::PageQuery,
    ) -> anyhow::Result<erp_repo::sku_grid::SkuGridPage> {
        self.calls.lock().unwrap().push((tenant, product));
        Ok(erp_repo::sku_grid::SkuGridPage {
            data: Vec::new(),
            total: 0,
        })
    }

    async fn query_sku_ranges(
        &self,
        tenant: TenantId,
        product: uuid::Uuid,
        request: &erp_repo::sku_grid::SkuGridRangesRequest,
    ) -> anyhow::Result<erp_repo::sku_grid::SkuGridRangesPage> {
        self.range_calls
            .lock()
            .unwrap()
            .push((tenant, product, request.clone()));
        Ok(erp_repo::sku_grid::SkuGridRangesPage {
            total: 0,
            pages: request
                .ranges
                .iter()
                .map(|range| erp_repo::sku_grid::SkuGridRangePage {
                    offset: range.offset,
                    data: Vec::new(),
                })
                .collect(),
        })
    }

    async fn list_skus(&self, _: TenantId, _: SkuListFilters) -> anyhow::Result<SkuListResult> {
        anyhow::bail!("unexpected")
    }
    async fn search_skus(
        &self,
        _: TenantId,
        _: SkuSearchFilters,
    ) -> anyhow::Result<Vec<RepoSkuSearchItem>> {
        anyhow::bail!("unexpected")
    }
    async fn get_sku(&self, _: TenantId, _: uuid::Uuid) -> anyhow::Result<Option<Sku>> {
        anyhow::bail!("unexpected")
    }
    async fn create_sku(&self, _: SkuCreate) -> anyhow::Result<Sku> {
        anyhow::bail!("unexpected")
    }
    async fn update_sku(
        &self,
        _: TenantId,
        _: uuid::Uuid,
        _: SkuUpdate,
    ) -> anyhow::Result<Option<Sku>> {
        anyhow::bail!("unexpected")
    }
    async fn delete_sku(&self, _: TenantId, _: uuid::Uuid) -> anyhow::Result<Option<Sku>> {
        anyhow::bail!("unexpected")
    }
}

#[derive(Default)]
struct Mappings;

#[async_trait]
impl ChannelSkuMappingRepo for Mappings {
    async fn list_mappings(
        &self,
        _: TenantId,
        _: ChannelSkuMappingListFilters,
    ) -> anyhow::Result<ChannelSkuMappingListResult> {
        anyhow::bail!("unexpected")
    }
    async fn create_mapping(
        &self,
        _: erp_repo::channel_sku_mappings::ChannelSkuMappingCreate,
    ) -> anyhow::Result<ChannelSkuMapping> {
        anyhow::bail!("unexpected")
    }
    async fn update_mapping(
        &self,
        _: TenantId,
        _: uuid::Uuid,
        _: erp_repo::channel_sku_mappings::ChannelSkuMappingUpdate,
    ) -> anyhow::Result<Option<ChannelSkuMapping>> {
        anyhow::bail!("unexpected")
    }
}

fn auth(tenant: TenantId, perms: &[&str]) -> AuthContext {
    AuthContext {
        tenant_id: tenant,
        user_id: UserId(uuid::Uuid::from_u128(2)),
        perms: normalize_permissions(perms.iter().map(|v| (*v).to_string()).collect()),
        is_admin: false,
        shop_ids: Vec::new(),
        warehouse_ids: Vec::new(),
        merchant_ids: Vec::new(),
        organization_ids: Vec::new(),
    session_id: None,
    }
}

fn body(sort_column: u32) -> serde_json::Value {
    serde_json::json!({
        "schema": 1,
        "offset": 0,
        "limit": 100,
        "sorts": [{"column": sort_column, "descending": false}]
    })
}

#[test]
fn sku_grid_read_only_whitelist_requires_exact_uuid_route() {
    let tenant = uuid::Uuid::from_u128(1);
    let product = uuid::Uuid::from_u128(3);
    let path = format!("/v1/tenants/{tenant}/products/{product}/skus/query");
    assert!(is_product_grid_read(&axum::http::Method::POST, &path));
    assert!(is_product_grid_read(
        &axum::http::Method::POST,
        &format!("/v1/tenants/{tenant}/products/{product}/skus/query-ranges")
    ));
    assert!(!is_product_grid_read(&axum::http::Method::GET, &path));
    assert!(!is_product_grid_read(
        &axum::http::Method::POST,
        "/v1/tenants/not-a-uuid/products/00000000-0000-0000-0000-000000000003/skus/query"
    ));
    assert!(!is_product_grid_read(
        &axum::http::Method::POST,
        &format!("{path}/extra")
    ));
    assert!(!is_product_grid_read(
        &axum::http::Method::POST,
        &format!("/v1/tenants/{tenant}/products/{product}/skus/query/action")
    ));
    assert!(!is_product_grid_read(
        &axum::http::Method::POST,
        &format!("/v1/tenants/{tenant}/skus/actions")
    ));
}

fn app(repo: Arc<Repo>, context: AuthContext) -> axum::Router {
    let db = Db::connect_lazy("postgres://localhost/unused").unwrap();
    let state = AppState::with_sku_repos(
        db,
        Arc::new(JwtService::new("fixture".into())),
        repo,
        Arc::new(Mappings),
    );
    axum::Router::new()
        .route(
            "/v1/tenants/:tenant_id/products/:product_id/skus/query",
            post(query_skus_grid),
        )
        .route(
            "/v1/tenants/:tenant_id/products/:product_id/skus/query-ranges",
            post(query_sku_ranges_grid),
        )
        .route("/v1/tenants/:tenant_id/skus/actions", post(sku_action))
        .route(
            "/v1/tenants/:tenant_id/skus/actions/:operation_id",
            axum::routing::get(sku_action_receipt),
        )
        .layer(Extension(context))
        .with_state(state)
}

fn action_body(value: &str) -> serde_json::Value {
    serde_json::json!({
        "operation_id": erp_grid_core::RowId(90),
        "action": "skus.update",
        "source": erp_grid_core::RowId(10),
        "expected_version": "2026-09-10T00:00:00Z",
        "values": {"4": value}
    })
}

#[tokio::test]
async fn sku_action_requires_read_and_write_and_rejects_bad_money_before_repo() {
    let repo = Arc::new(Repo::default());
    let tenant = TenantId(uuid::Uuid::from_u128(1));
    for permissions in [&["skus.read"][..], &["skus.write"][..]] {
        let response = app(repo.clone(), auth(tenant, permissions))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tenants/{tenant}/skus/actions"))
                    .header("content-type", "application/json")
                    .body(Body::from(action_body("1.00").to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let response = app(repo.clone(), auth(tenant, &["skus.read", "skus.write"]))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/tenants/{tenant}/skus/actions"))
                .header("content-type", "application/json")
                .body(Body::from(action_body("1.001").to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(repo.action_calls.lock().unwrap().is_empty());

    let response = app(repo.clone(), auth(tenant, &["skus.read", "skus.write"]))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/tenants/{tenant}/skus/actions"))
                .header("content-type", "application/json")
                .body(Body::from(action_body("1.00").to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let calls = repo.action_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        (calls[0].0, calls[0].1),
        (tenant, UserId(uuid::Uuid::from_u128(2)))
    );
}

#[tokio::test]
async fn sku_grid_checks_permission_and_validates_columns_before_repo() {
    let repo = Arc::new(Repo::default());
    let tenant = TenantId(uuid::Uuid::from_u128(1));
    let product = uuid::Uuid::from_u128(3);
    let response = app(repo.clone(), auth(tenant, &[]))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant}/products/{product}/skus/query"
                ))
                .header("content-type", "application/json")
                .body(Body::from(body(0).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(repo.calls.lock().unwrap().is_empty());

    let response = app(repo.clone(), auth(tenant, &["skus.read"]))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{}/products/{product}/skus/query",
                    TenantId(uuid::Uuid::from_u128(9))
                ))
                .header("content-type", "application/json")
                .body(Body::from(body(0).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(repo.calls.lock().unwrap().is_empty());

    let response = app(repo.clone(), auth(tenant, &["skus.read"]))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant}/products/{product}/skus/query"
                ))
                .header("content-type", "application/json")
                .body(Body::from(body(999).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(repo.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn sku_grid_passes_tenant_and_parent_product_to_repo() {
    let repo = Arc::new(Repo::default());
    let tenant = TenantId(uuid::Uuid::from_u128(1));
    let product = uuid::Uuid::from_u128(3);
    let response = app(repo.clone(), auth(tenant, &["skus.read"]))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant}/products/{product}/skus/query"
                ))
                .header("content-type", "application/json")
                .body(Body::from(body(0).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["total"], 0);
    assert_eq!(repo.calls.lock().unwrap().as_slice(), &[(tenant, product)]);
}

#[tokio::test]
async fn sku_ranges_validate_bounds_and_pass_fixed_query_to_repo() {
    let repo = Arc::new(Repo::default());
    let tenant = TenantId(uuid::Uuid::from_u128(1));
    let product = uuid::Uuid::from_u128(3);
    let uri = format!("/v1/tenants/{tenant}/products/{product}/skus/query-ranges");
    let invalid = serde_json::json!({
        "query": body(0),
        "ranges": [{"offset": 0, "limit": 1001}]
    });
    let response = app(repo.clone(), auth(tenant, &["skus.read"]))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&uri)
                .header("content-type", "application/json")
                .body(Body::from(invalid.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(repo.range_calls.lock().unwrap().is_empty());

    let valid = serde_json::json!({
        "query": body(0),
        "ranges": [{"offset": 10, "limit": 20}, {"offset": 15, "limit": 30}]
    });
    let response = app(repo.clone(), auth(tenant, &["skus.read"]))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&uri)
                .header("content-type", "application/json")
                .body(Body::from(valid.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let calls = repo.range_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!((calls[0].0, calls[0].1), (tenant, product));
    assert_eq!(calls[0].2.ranges[1].offset, 15);
}

#[tokio::test]
async fn sku_action_receipt_uses_authenticated_actor_and_read_permission() {
    let repo = Arc::new(Repo::default());
    let tenant = TenantId(uuid::Uuid::from_u128(1));
    let operation = erp_grid_core::RowId(90);
    let uri = format!("/v1/tenants/{tenant}/skus/actions/{}", operation.to_uuid());
    let forbidden = app(repo.clone(), auth(tenant, &[]))
        .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    assert!(repo.receipt_calls.lock().unwrap().is_empty());

    let missing = app(repo.clone(), auth(tenant, &["skus.read"]))
        .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        repo.receipt_calls.lock().unwrap().as_slice(),
        &[(tenant, UserId(uuid::Uuid::from_u128(2)), operation)]
    );
}
