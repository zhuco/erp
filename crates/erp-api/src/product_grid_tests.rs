use super::*;
use axum::body::Body;
use erp_grid_core::{ColumnId, Filter, paged_query::PageQuery};
use erp_repo::product_grid::{ProductGridPage, ProductGridRow};
use http_body_util::BodyExt;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;

#[derive(Clone, Debug, PartialEq, Eq)]
enum BatchCall {
    Results(TenantId, UserId, uuid::Uuid, Option<u64>, u16, bool),
    List(TenantId, UserId),
    Start(
        TenantId,
        UserId,
        uuid::Uuid,
        uuid::Uuid,
        Vec<uuid::Uuid>,
        u64,
    ),
    Status(TenantId, UserId, uuid::Uuid),
    Advance(TenantId, UserId, uuid::Uuid, u16),
    Cancel(TenantId, UserId, uuid::Uuid),
    RetryReview(TenantId, UserId, uuid::Uuid, uuid::Uuid),
}

struct Repo(
    AtomicUsize,
    Arc<Mutex<Option<(TenantId, UserId)>>>,
    Arc<Mutex<Vec<BatchCall>>>,
);

impl Repo {
    fn new() -> Self {
        Self(
            AtomicUsize::new(0),
            Arc::new(Mutex::new(None)),
            Arc::new(Mutex::new(Vec::new())),
        )
    }
}

fn batch_status(batch_id: uuid::Uuid) -> erp_repo::product_copy_batches::Status {
    erp_repo::product_copy_batches::Status {
        batch_id,
        total: 4,
        applied: 1,
        rejected: 0,
        cancelled: 0,
        pending: 3,
        unknown: 0,
        cancel_requested: false,
    }
}

#[tokio::test]
async fn product_options_handler_authorizes_and_validates_before_query() {
    let repo = Arc::new(Repo::new());
    let tenant = TenantId(uuid::Uuid::from_u128(1));
    let base = serde_json::json!({"query":{"schema":1,"offset":0,"limit":100},"column":2,"search":null,"offset":0,"limit":100});
    let mut invalid = base.clone();
    invalid["column"] = serde_json::json!(99);
    for (path_tenant, allowed, body, expected) in [
        (tenant, true, base.clone(), StatusCode::OK),
        (tenant, false, base.clone(), StatusCode::FORBIDDEN),
        (
            TenantId(uuid::Uuid::from_u128(3)),
            true,
            base.clone(),
            StatusCode::FORBIDDEN,
        ),
        (tenant, true, invalid, StatusCode::BAD_REQUEST),
    ] {
        let auth = AuthContext {
            tenant_id: tenant,
            user_id: UserId(uuid::Uuid::from_u128(2)),
            perms: if allowed {
                normalize_permissions(vec!["skus.read".into()])
            } else {
                vec![]
            },
            is_admin: false,
            shop_ids: vec![],
            warehouse_ids: vec![],
            merchant_ids: vec![],
            organization_ids: vec![],
        };
        let router = Router::new()
            .route(
                "/v1/tenants/:tenant_id/products/options",
                post(product_options),
            )
            .layer(Extension(auth))
            .with_state(state(repo.clone()));
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tenants/{path_tenant}/products/options"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        if expected == StatusCode::OK {
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["values"][0], "");
            assert_eq!(body["total"], 2);
        }
    }
    assert_eq!(repo.0.load(Ordering::SeqCst), 1);
    assert!(auth_layers::is_product_grid_read(
        &axum::http::Method::POST,
        &format!("/v1/tenants/{tenant}/products/options")
    ));
    assert!(!auth_layers::is_product_grid_read(
        &axum::http::Method::POST,
        &format!("/v1/tenants/{tenant}/products/actions")
    ));
}

#[async_trait]
impl ProductRepo for Repo {
    async fn retry_copy_batch_review(
        &self,
        tenant: TenantId,
        actor: UserId,
        original_batch: uuid::Uuid,
        request_id: uuid::Uuid,
    ) -> Result<
        erp_repo::product_copy_batch_retries::RetryReview,
        erp_repo::product_copy_batch_retries::RetryError,
    > {
        *self.1.lock().unwrap() = Some((tenant, actor));
        self.2.lock().unwrap().push(BatchCall::RetryReview(
            tenant,
            actor,
            original_batch,
            request_id,
        ));
        Ok(erp_repo::product_copy_batch_retries::RetryReview {
            batch_id: original_batch,
            request_id,
            selection: erp_repo::product_selection::SelectionToken {
                token: uuid::Uuid::from_u128(901),
                query_fingerprint: "fixture".into(),
                total: 2,
                expires_at: 999,
            },
        })
    }
    async fn copy_batch_results(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
        after: Option<u64>,
        limit: u16,
        failed_only: bool,
    ) -> Result<
        erp_repo::product_copy_batch_results::Results,
        erp_repo::product_copy_batches::BatchError,
    > {
        *self.1.lock().unwrap() = Some((tenant, actor));
        self.2.lock().unwrap().push(BatchCall::Results(
            tenant,
            actor,
            batch_id,
            after,
            limit,
            failed_only,
        ));
        Ok(erp_repo::product_copy_batch_results::Results {
            batch_id,
            items: vec![],
            next_after: None,
        })
    }
    async fn list_copy_batches(
        &self,
        tenant: TenantId,
        actor: UserId,
    ) -> Result<
        Vec<erp_repo::product_copy_batches::Status>,
        erp_repo::product_copy_batches::BatchError,
    > {
        *self.1.lock().unwrap() = Some((tenant, actor));
        self.2.lock().unwrap().push(BatchCall::List(tenant, actor));
        Ok(vec![batch_status(uuid::Uuid::from_u128(700))])
    }
    async fn start_copy_batch(
        &self,
        tenant: TenantId,
        actor: UserId,
        request: &erp_repo::product_copy_batches::StartRequest,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>
    {
        self.0.fetch_add(1, Ordering::SeqCst);
        *self.1.lock().unwrap() = Some((tenant, actor));
        self.2.lock().unwrap().push(BatchCall::Start(
            tenant,
            actor,
            request.batch_id,
            request.token,
            request.excluded.clone(),
            request.confirmed_count,
        ));
        Ok(batch_status(request.batch_id))
    }
    async fn copy_batch_status(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>
    {
        *self.1.lock().unwrap() = Some((tenant, actor));
        self.2
            .lock()
            .unwrap()
            .push(BatchCall::Status(tenant, actor, batch_id));
        Ok(batch_status(batch_id))
    }
    async fn advance_copy_batch(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
        max_items: u16,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>
    {
        self.0.fetch_add(1, Ordering::SeqCst);
        *self.1.lock().unwrap() = Some((tenant, actor));
        self.2
            .lock()
            .unwrap()
            .push(BatchCall::Advance(tenant, actor, batch_id, max_items));
        Ok(batch_status(batch_id))
    }
    async fn cancel_copy_batch(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>
    {
        self.0.fetch_add(1, Ordering::SeqCst);
        *self.1.lock().unwrap() = Some((tenant, actor));
        self.2
            .lock()
            .unwrap()
            .push(BatchCall::Cancel(tenant, actor, batch_id));
        Ok(batch_status(batch_id))
    }
    async fn freeze_selection(
        &self,
        tenant: TenantId,
        actor: UserId,
        _: &PageQuery,
    ) -> Result<
        erp_repo::product_selection::SelectionToken,
        erp_repo::product_selection::SelectionError,
    > {
        self.0.fetch_add(1, Ordering::SeqCst);
        *self.1.lock().unwrap() = Some((tenant, actor));
        Ok(erp_repo::product_selection::SelectionToken {
            token: uuid::Uuid::from_u128(20),
            query_fingerprint: "fixture".into(),
            total: 1,
            expires_at: 2,
        })
    }
    async fn selection_page(
        &self,
        tenant: TenantId,
        actor: UserId,
        token: uuid::Uuid,
        _: u64,
        _: u16,
        sorts: &[erp_grid_core::SortRule],
    ) -> Result<
        erp_repo::product_selection::SelectionPage,
        erp_repo::product_selection::SelectionError,
    > {
        self.0.fetch_add(1, Ordering::SeqCst);
        *self.1.lock().unwrap() = Some((tenant, actor));
        if token == uuid::Uuid::from_u128(20) {
            assert_eq!(
                sorts,
                &[erp_grid_core::SortRule {
                    column: ColumnId(2),
                    descending: true,
                }]
            );
        }
        if token.is_nil() {
            return Err(erp_repo::product_selection::SelectionError::Unavailable);
        }
        Ok(erp_repo::product_selection::SelectionPage {
            data: vec![serde_json::json!({"product_id": uuid::Uuid::from_u128(21)})],
            total: 1,
            token,
            expires_at: 2,
        })
    }
    async fn selection_count(
        &self,
        tenant: TenantId,
        actor: UserId,
        _: uuid::Uuid,
        _: &[uuid::Uuid],
    ) -> Result<u64, erp_repo::product_selection::SelectionError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        *self.1.lock().unwrap() = Some((tenant, actor));
        Ok(1)
    }
    async fn product_options(
        &self,
        _: TenantId,
        query: &erp_repo::product_grid_options::OptionQuery,
    ) -> anyhow::Result<erp_repo::product_grid_options::OptionPage> {
        self.0.fetch_add(1, Ordering::SeqCst);
        assert_eq!(query.column, ColumnId(2));
        Ok(erp_repo::product_grid_options::OptionPage {
            values: vec![String::new(), "Brand from another page".into()],
            total: 2,
        })
    }
    async fn execute_action(
        &self,
        _: TenantId,
        _: UserId,
        command: &erp_grid_core::actions::ActionCommand,
    ) -> Result<erp_grid_core::actions::ActionReceipt, erp_repo::product_actions::ActionError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if command.operation_id == erp_grid_core::RowId(98) {
            return Err(erp_repo::product_actions::ActionError::IdentityConflict);
        }
        Ok(erp_grid_core::actions::ActionReceipt {
            operation_id: command.operation_id,
            source: command.source,
            command_fingerprint: command.fingerprint(),
            outcome: erp_grid_core::actions::ActionOutcome::Applied {
                row: erp_grid_core::RowId(200),
                version: "2026-09-09T00:00:00Z".into(),
                values: Default::default(),
            },
        })
    }
    async fn action_receipt(
        &self,
        _: TenantId,
        _: UserId,
        operation: erp_grid_core::RowId,
    ) -> Result<Option<erp_grid_core::actions::ActionReceipt>, erp_repo::product_actions::ActionError>
    {
        self.0.fetch_add(1, Ordering::SeqCst);
        if operation == erp_grid_core::RowId(97) {
            return Err(erp_repo::product_actions::ActionError::ReceiptMismatch);
        }
        Ok(
            (operation == erp_grid_core::RowId(99)).then(|| {
                erp_grid_core::actions::ActionReceipt {
                    operation_id: operation,
                    source: erp_grid_core::RowId(10),
                    command_fingerprint: erp_grid_core::actions::ActionCommand {
                        operation_id: operation,
                        action: "products.copy".into(),
                        source: erp_grid_core::RowId(10),
                        expected_version: None,
                        values: Default::default(),
                    }
                    .fingerprint(),
                    outcome: erp_grid_core::actions::ActionOutcome::Applied {
                        row: erp_grid_core::RowId(200),
                        version: "2026-09-09T00:00:00Z".into(),
                        values: Default::default(),
                    },
                }
            }),
        )
    }
    async fn query_products(
        &self,
        tenant: TenantId,
        query: &PageQuery,
    ) -> anyhow::Result<ProductGridPage> {
        self.0.fetch_add(1, Ordering::SeqCst);
        assert_eq!((query.offset, query.limit), (9900, 100));
        assert_eq!(
            query.filters.get(&ColumnId(1)),
            Some(&Filter::Text("needle".into()))
        );
        Ok(ProductGridPage {
            total: 10000,
            data: (0..100)
                .map(|index| ProductGridRow {
                    product_id: uuid::Uuid::from_u128(index + 9901),
                    tenant_id: tenant.into(),
                    product_code: None,
                    name: "needle".into(),
                    brand: None,
                    category: None,
                    status: "ACTIVE".into(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
                .collect(),
        })
    }
    async fn list_products(
        &self,
        _: TenantId,
        _: ProductListFilters,
    ) -> anyhow::Result<ProductListResult> {
        anyhow::bail!("unexpected list")
    }
    async fn get_product(&self, _: TenantId, _: uuid::Uuid) -> anyhow::Result<Option<Product>> {
        anyhow::bail!("unexpected get")
    }
    async fn create_product(&self, _: ProductCreate) -> anyhow::Result<Product> {
        anyhow::bail!("unexpected write")
    }
    async fn update_product(
        &self,
        _: TenantId,
        _: uuid::Uuid,
        _: ProductUpdate,
    ) -> anyhow::Result<Option<Product>> {
        anyhow::bail!("unexpected write")
    }
    async fn delete_product(&self, _: TenantId, _: uuid::Uuid) -> anyhow::Result<Option<Product>> {
        anyhow::bail!("unexpected write")
    }
}

fn state(repo: Arc<Repo>) -> AppState {
    // No database connection is made by these handler tests.
    let mut state = AppState::new(
        Db::connect_lazy("postgres://localhost/unused").unwrap(),
        Arc::new(JwtService::new("fixture-only".into())),
    );
    state.products_repo = repo;
    state
}
fn query_body() -> Value {
    serde_json::json!({"schema": 1, "offset": 9900, "limit": 100, "filters": {"1": {"Text": "needle"}}, "sorts": [{"column": 1, "descending": false}]})
}

#[tokio::test]
async fn product_selection_handlers_forward_identity_and_reject_denied_context() {
    let repo = Arc::new(Repo::new());
    let tenant = TenantId(uuid::Uuid::from_u128(31));
    let actor = UserId(uuid::Uuid::from_u128(32));
    let auth = AuthContext {
        tenant_id: tenant,
        user_id: actor,
        perms: normalize_permissions(vec!["skus.read".into(), "skus.write".into()]),
        is_admin: false,
        shop_ids: vec![],
        warehouse_ids: vec![],
        merchant_ids: vec![],
        organization_ids: vec![],
    };
    let app = Router::new()
        .route(
            "/v1/tenants/:tenant_id/products/selections",
            post(freeze_product_selection),
        )
        .route(
            "/v1/tenants/:tenant_id/products/selections/:token/page",
            post(product_selection_page),
        )
        .route(
            "/v1/tenants/:tenant_id/products/selections/:token/count",
            post(product_selection_count),
        )
        .layer(Extension(auth.clone()))
        .with_state(state(repo.clone()));
    let query = serde_json::json!({
        "schema": 1, "offset": 0, "limit": 10, "filters": {}, "sorts": []
    });
    let response = app
        .clone()
        .oneshot(json_request(
            &format!("/v1/tenants/{tenant}/products/selections"),
            query,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    for sorts in [
        serde_json::json!([
            {"column": 1, "descending": false},
            {"column": 1, "descending": true}
        ]),
        serde_json::json!([{"column": 7, "descending": false}]),
    ] {
        let response = app
            .clone()
            .oneshot(json_request(
                &format!(
                    "/v1/tenants/{tenant}/products/selections/{}/page",
                    uuid::Uuid::from_u128(20)
                ),
                serde_json::json!({"offset": 0, "limit": 10, "sorts": sorts}),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let response = app
        .clone()
        .oneshot(json_request(
            &format!(
                "/v1/tenants/{tenant}/products/selections/{}/page",
                uuid::Uuid::from_u128(20)
            ),
            serde_json::json!({"offset": 0, "limit": 10, "sorts": [{"column": 2, "descending": true}]}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .clone()
        .oneshot(json_request(
            &format!(
                "/v1/tenants/{tenant}/products/selections/{}/count",
                uuid::Uuid::from_u128(20)
            ),
            serde_json::json!({"excluded": []}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(*repo.1.lock().unwrap(), Some((tenant, actor)));
    assert_eq!(repo.0.load(Ordering::SeqCst), 3);

    let denied = AuthContext {
        perms: vec![],
        ..auth
    };
    let denied_app = Router::new()
        .route(
            "/v1/tenants/:tenant_id/products/selections",
            post(freeze_product_selection),
        )
        .layer(Extension(denied))
        .with_state(state(repo.clone()));
    let response = denied_app
        .oneshot(json_request(
            &format!("/v1/tenants/{tenant}/products/selections"),
            serde_json::json!({"schema": 1, "offset": 0, "limit": 10}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(repo.0.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn product_selection_unavailable_maps_to_gone() {
    let repo = Arc::new(Repo::new());
    let tenant = TenantId(uuid::Uuid::from_u128(41));
    let auth = AuthContext {
        tenant_id: tenant,
        user_id: UserId(uuid::Uuid::from_u128(42)),
        perms: normalize_permissions(vec!["skus.read".into(), "skus.write".into()]),
        is_admin: false,
        shop_ids: vec![],
        warehouse_ids: vec![],
        merchant_ids: vec![],
        organization_ids: vec![],
    };
    let app = Router::new()
        .route(
            "/v1/tenants/:tenant_id/products/selections/:token/page",
            post(product_selection_page),
        )
        .layer(Extension(auth))
        .with_state(state(repo));
    let response = app
        .oneshot(json_request(
            &format!(
                "/v1/tenants/{tenant}/products/selections/{}/page",
                uuid::Uuid::nil()
            ),
            serde_json::json!({"offset": 0, "limit": 10}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::GONE);
}

fn json_request(path: &str, body: Value) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn copy_batch_routes_validate_before_repo_and_keep_post_write_guard() {
    let repo = Arc::new(Repo::new());
    let tenant = TenantId(uuid::Uuid::from_u128(71));
    let actor = UserId(uuid::Uuid::from_u128(72));
    let auth = AuthContext {
        tenant_id: tenant,
        user_id: actor,
        perms: normalize_permissions(vec!["skus.read".into(), "skus.write".into()]),
        is_admin: false,
        shop_ids: vec![],
        warehouse_ids: vec![],
        merchant_ids: vec![],
        organization_ids: vec![],
    };
    let app = Router::new()
        .route(
            "/v1/tenants/:tenant_id/products/copy-batches",
            get(list_product_copy_batches).post(start_product_copy_batch),
        )
        .route(
            "/v1/tenants/:tenant_id/products/copy-batches/:batch_id/advance",
            post(advance_product_copy_batch),
        )
        .layer(Extension(auth.clone()))
        .with_state(state(repo.clone()));
    let base = format!("/v1/tenants/{tenant}/products/copy-batches");
    let invalid_start = serde_json::json!({
        "batch_id": uuid::Uuid::nil(), "token": uuid::Uuid::from_u128(2),
        "excluded": [], "confirmed_count": 1
    });
    let response = app
        .clone()
        .oneshot(json_request(&base, invalid_start))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(repo.0.load(Ordering::SeqCst), 0);

    let response = app
        .clone()
        .oneshot(json_request(
            &base,
            serde_json::json!({
                "batch_id": uuid::Uuid::from_u128(1), "token": uuid::Uuid::from_u128(2),
                "excluded": [uuid::Uuid::from_u128(3), uuid::Uuid::from_u128(3)], "confirmed_count": 1
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(repo.0.load(Ordering::SeqCst), 0);

    let denied_app = Router::new()
        .route(
            "/v1/tenants/:tenant_id/products/copy-batches",
            post(start_product_copy_batch),
        )
        .layer(Extension(AuthContext {
            perms: normalize_permissions(vec!["skus.read".into()]),
            ..auth
        }))
        .with_state(state(repo.clone()));
    let response = denied_app
        .oneshot(json_request(
            &base,
            serde_json::json!({
                "batch_id": uuid::Uuid::from_u128(1), "token": uuid::Uuid::from_u128(2),
                "excluded": [], "confirmed_count": 1
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(repo.0.load(Ordering::SeqCst), 0);

    let response = app
        .clone()
        .oneshot(json_request(
            &format!("{base}/{}/advance", uuid::Uuid::from_u128(1)),
            serde_json::json!({"max_items": 26}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(repo.0.load(Ordering::SeqCst), 0);

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri(&base)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(repo.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn copy_batch_valid_routes_forward_identity_and_keep_reads_non_mutating() {
    let repo = Arc::new(Repo::new());
    let tenant = TenantId(uuid::Uuid::from_u128(81));
    let actor = UserId(uuid::Uuid::from_u128(82));
    let batch = uuid::Uuid::from_u128(83);
    let token = uuid::Uuid::from_u128(84);
    let excluded = [uuid::Uuid::from_u128(85), uuid::Uuid::from_u128(86)];
    let auth = AuthContext {
        tenant_id: tenant,
        user_id: actor,
        perms: normalize_permissions(vec!["skus.read".into(), "skus.write".into()]),
        is_admin: false,
        shop_ids: vec![],
        warehouse_ids: vec![],
        merchant_ids: vec![],
        organization_ids: vec![],
    };
    let routes = || {
        Router::new()
            .route(
                "/v1/tenants/:tenant_id/products/copy-batches",
                get(list_product_copy_batches).post(start_product_copy_batch),
            )
            .route(
                "/v1/tenants/:tenant_id/products/copy-batches/:batch_id",
                get(product_copy_batch_status),
            )
            .route(
                "/v1/tenants/:tenant_id/products/copy-batches/:batch_id/advance",
                post(advance_product_copy_batch),
            )
            .route(
                "/v1/tenants/:tenant_id/products/copy-batches/:batch_id/cancel",
                post(cancel_product_copy_batch),
            )
    };
    let app = routes()
        .layer(Extension(auth.clone()))
        .with_state(state(repo.clone()));
    let base = format!("/v1/tenants/{tenant}/products/copy-batches");

    for (index, path) in [base.clone(), format!("{base}/{batch}")]
        .into_iter()
        .enumerate()
    {
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        if index == 0 {
            assert_eq!(body[0]["batch_id"], uuid::Uuid::from_u128(700).to_string());
        } else {
            assert_eq!(body["batch_id"], batch.to_string());
        }
    }
    assert_eq!(repo.0.load(Ordering::SeqCst), 0);

    let start = app
        .clone()
        .oneshot(json_request(
            &base,
            serde_json::json!({
                "batch_id": batch,
                "token": token,
                "excluded": excluded,
                "confirmed_count": 4
            }),
        ))
        .await
        .unwrap();
    assert_eq!(start.status(), StatusCode::OK);
    let advance = app
        .clone()
        .oneshot(json_request(
            &format!("{base}/{batch}/advance"),
            serde_json::json!({"max_items": 25}),
        ))
        .await
        .unwrap();
    assert_eq!(advance.status(), StatusCode::OK);
    let cancel = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("{base}/{batch}/cancel"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cancel.status(), StatusCode::OK);
    assert_eq!(repo.0.load(Ordering::SeqCst), 3);
    assert_eq!(
        *repo.2.lock().unwrap(),
        vec![
            BatchCall::List(tenant, actor),
            BatchCall::Status(tenant, actor, batch),
            BatchCall::Start(tenant, actor, batch, token, excluded.to_vec(), 4),
            BatchCall::Advance(tenant, actor, batch, 25),
            BatchCall::Cancel(tenant, actor, batch),
        ]
    );

    let read_only = routes()
        .layer(Extension(AuthContext {
            perms: normalize_permissions(vec!["skus.read".into()]),
            ..auth
        }))
        .with_state(state(repo.clone()));
    for path in [base.clone(), format!("{base}/{batch}")] {
        let response = read_only
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(repo.0.load(Ordering::SeqCst), 3);
    assert!(matches!(
        repo.2.lock().unwrap().as_slice(),
        [.., BatchCall::List(t, a), BatchCall::Status(t2, a2, b)]
            if (*t, *a, *t2, *a2, *b) == (tenant, actor, tenant, actor, batch)
    ));
    let mutation_count = repo.0.load(Ordering::SeqCst);
    let recorded_count = repo.2.lock().unwrap().len();
    let denied = [
        json_request(
            &base,
            serde_json::json!({
                "batch_id": batch,
                "token": token,
                "excluded": [],
                "confirmed_count": 4
            }),
        ),
        json_request(
            &format!("{base}/{batch}/advance"),
            serde_json::json!({"max_items": 25}),
        ),
        axum::http::Request::builder()
            .method("POST")
            .uri(format!("{base}/{batch}/cancel"))
            .body(Body::empty())
            .unwrap(),
    ];
    for request in denied {
        let response = read_only.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    assert_eq!(repo.0.load(Ordering::SeqCst), mutation_count);
    assert_eq!(repo.2.lock().unwrap().len(), recorded_count);
}

#[tokio::test]
async fn copy_batch_retry_review_forwards_identity_and_guards_request_shape() {
    let repo = Arc::new(Repo::new());
    let tenant = TenantId(uuid::Uuid::from_u128(811));
    let actor = UserId(uuid::Uuid::from_u128(812));
    let batch = uuid::Uuid::from_u128(813);
    let request_id = uuid::Uuid::from_u128(814);
    let auth = AuthContext {
        tenant_id: tenant,
        user_id: actor,
        perms: normalize_permissions(vec!["skus.read".into(), "skus.write".into()]),
        is_admin: false,
        shop_ids: vec![],
        warehouse_ids: vec![],
        merchant_ids: vec![],
        organization_ids: vec![],
    };
    let app = Router::new()
        .route(
            "/v1/tenants/:tenant_id/products/copy-batches/:batch_id/retry-review",
            post(retry_product_copy_batch_review),
        )
        .layer(Extension(auth.clone()))
        .with_state(state(repo.clone()));
    let path = format!("/v1/tenants/{tenant}/products/copy-batches/{batch}/retry-review");
    let response = app
        .clone()
        .oneshot(json_request(
            &path,
            serde_json::json!({"request_id": request_id}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["batch_id"], batch.to_string());
    assert_eq!(body["request_id"], request_id.to_string());
    assert_eq!(
        repo.2.lock().unwrap().as_slice(),
        &[BatchCall::RetryReview(tenant, actor, batch, request_id)]
    );

    let response = app
        .clone()
        .oneshot(json_request(
            &path,
            serde_json::json!({"request_id": uuid::Uuid::nil()}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = app
        .clone()
        .oneshot(json_request(
            &path,
            serde_json::json!({"request_id": request_id, "extra": true}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let denied = Router::new()
        .route(
            "/v1/tenants/:tenant_id/products/copy-batches/:batch_id/retry-review",
            post(retry_product_copy_batch_review),
        )
        .layer(Extension(AuthContext {
            perms: normalize_permissions(vec!["skus.read".into()]),
            ..auth
        }))
        .with_state(state(repo.clone()));
    let response = denied
        .oneshot(json_request(
            &path,
            serde_json::json!({"request_id": uuid::Uuid::from_u128(815)}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(repo.2.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn copy_batch_results_get_forwards_read_only_page_arguments_once() {
    let repo = Arc::new(Repo::new());
    let tenant = TenantId(uuid::Uuid::from_u128(91));
    let actor = UserId(uuid::Uuid::from_u128(92));
    let batch = uuid::Uuid::from_u128(93);
    let app = Router::new()
        .route(
            "/v1/tenants/:tenant_id/products/copy-batches/:batch_id/results",
            get(product_copy_batch_results),
        )
        .layer(Extension(AuthContext {
            tenant_id: tenant,
            user_id: actor,
            perms: normalize_permissions(vec!["skus.read".into()]),
            is_admin: false,
            shop_ids: vec![],
            warehouse_ids: vec![],
            merchant_ids: vec![],
            organization_ids: vec![],
        }))
        .with_state(state(repo.clone()));
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri(format!(
                    "/v1/tenants/{tenant}/products/copy-batches/{batch}/results?after=7&limit=25&failed_only=true"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["batch_id"], batch.to_string());
    assert_eq!(repo.0.load(Ordering::SeqCst), 0);
    assert_eq!(
        *repo.2.lock().unwrap(),
        vec![BatchCall::Results(tenant, actor, batch, Some(7), 25, true)]
    );
}

#[tokio::test]
async fn product_grid_handler_checks_tenant_permissions_and_wire_before_repo() {
    let repo = Arc::new(Repo::new());
    let tenant = TenantId(uuid::Uuid::from_u128(1));
    let auth = AuthContext {
        tenant_id: tenant,
        user_id: UserId(uuid::Uuid::from_u128(2)),
        perms: normalize_permissions(vec!["skus.read".into()]),
        is_admin: false,
        shop_ids: vec![],
        warehouse_ids: vec![],
        merchant_ids: vec![],
        organization_ids: vec![],
    };
    for (path_tenant, perms, body, expected) in [
        (tenant, auth.perms.clone(), query_body(), StatusCode::OK),
        (
            TenantId(uuid::Uuid::from_u128(3)),
            auth.perms.clone(),
            query_body(),
            StatusCode::FORBIDDEN,
        ),
        (tenant, vec![], query_body(), StatusCode::FORBIDDEN),
        (
            tenant,
            auth.perms.clone(),
            serde_json::json!({"schema": 1, "offset": 0, "limit": 501}),
            StatusCode::BAD_REQUEST,
        ),
        (
            tenant,
            auth.perms.clone(),
            serde_json::json!({"schema": 1, "offset": 0, "limit": 100, "sorts": [{"column": 999, "descending": false}]}),
            StatusCode::BAD_REQUEST,
        ),
        (
            tenant,
            auth.perms.clone(),
            serde_json::json!({"schema": 1, "offset": 0, "limit": 100, "raw_sql": "select"}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let router = Router::new()
            .route(
                "/v1/tenants/:tenant_id/products/query",
                post(query_products),
            )
            .layer(Extension(AuthContext {
                perms,
                ..auth.clone()
            }))
            .with_state(state(repo.clone()));
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tenants/{path_tenant}/products/query"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        if expected == StatusCode::OK {
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            let page: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(page["total"], 10000);
            assert_eq!(page["data"].as_array().unwrap().len(), 100);
            assert_eq!(
                page["data"][0]["product_id"],
                uuid::Uuid::from_u128(9901).to_string()
            );
        }
    }
    assert_eq!(repo.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn product_grid_real_router_requires_jwt_and_allows_read_post() {
    let repo = Arc::new(Repo::new());
    let state = state(repo.clone());
    let path = format!("/v1/tenants/{}/products/query", uuid::Uuid::from_u128(1));
    let now = Utc::now().timestamp() as usize;
    let token = state
        .jwt
        .encode(&Claims {
            sub: uuid::Uuid::from_u128(2).to_string(),
            tenant_id: uuid::Uuid::from_u128(1).to_string(),
            perms: vec![],
            is_admin: true,
            iat: now,
            exp: now + 3600,
        })
        .unwrap();
    for token in [None, Some(token)] {
        let mut request = axum::http::Request::builder()
            .method("POST")
            .uri(&path)
            .header("content-type", "application/json");
        if let Some(token) = &token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let response = build_app(state.clone())
            .oneshot(request.body(Body::from(query_body().to_string())).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            if token.is_some() {
                StatusCode::OK
            } else {
                StatusCode::UNAUTHORIZED
            }
        );
    }
    assert!(auth_layers::is_product_grid_read(
        &axum::http::Method::POST,
        &path
    ));
    assert!(!auth_layers::is_product_grid_read(
        &axum::http::Method::PUT,
        &path
    ));
    assert!(!auth_layers::is_product_grid_read(
        &axum::http::Method::POST,
        &(path + "/write")
    ));
    assert_eq!(repo.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn product_actions_http_permissions_receipts_and_identity_conflicts() {
    use erp_grid_core::{
        RowId,
        actions::{ActionCommand, ActionReceipt},
    };
    let repo = Arc::new(Repo::new());
    let tenant = TenantId(uuid::Uuid::from_u128(1));
    let auth = AuthContext {
        tenant_id: tenant,
        user_id: UserId(uuid::Uuid::from_u128(99)),
        perms: normalize_permissions(vec!["skus.read".into(), "skus.write".into()]),
        is_admin: false,
        shop_ids: vec![],
        warehouse_ids: vec![],
        merchant_ids: vec![],
        organization_ids: vec![],
    };
    let copy = ActionCommand {
        operation_id: RowId(99),
        action: "products.copy".into(),
        source: RowId(10),
        expected_version: None,
        values: Default::default(),
    };
    for (path_tenant, permissions, action, operation, expected) in [
        (
            tenant,
            auth.perms.clone(),
            "products.copy",
            99,
            StatusCode::OK,
        ),
        (
            tenant,
            normalize_permissions(vec!["skus.read".into()]),
            "products.copy",
            99,
            StatusCode::FORBIDDEN,
        ),
        (
            TenantId(uuid::Uuid::from_u128(2)),
            auth.perms.clone(),
            "products.copy",
            99,
            StatusCode::FORBIDDEN,
        ),
        (
            tenant,
            auth.perms.clone(),
            "products.delete",
            99,
            StatusCode::BAD_REQUEST,
        ),
        (
            tenant,
            auth.perms.clone(),
            "products.copy",
            98,
            StatusCode::CONFLICT,
        ),
    ] {
        let router = Router::new()
            .route(
                "/v1/tenants/:tenant_id/products/actions",
                post(product_action),
            )
            .layer(Extension(AuthContext {
                perms: permissions,
                ..auth.clone()
            }))
            .with_state(state(repo.clone()));
        let command = ActionCommand {
            action: action.into(),
            operation_id: RowId(operation),
            ..copy.clone()
        };
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tenants/{path_tenant}/products/actions"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&command).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        if expected == StatusCode::OK {
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            let result: ActionReceipt = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(result.command_fingerprint, command.fingerprint());
            assert_eq!(
                (result.operation_id, result.source),
                (copy.operation_id, copy.source)
            );
        }
    }
    assert_eq!(repo.0.load(Ordering::SeqCst), 2);
    for (operation, expected) in [
        (99, StatusCode::OK),
        (100, StatusCode::NOT_FOUND),
        (97, StatusCode::SERVICE_UNAVAILABLE),
    ] {
        let router = Router::new()
            .route(
                "/v1/tenants/:tenant_id/products/actions/:operation_id",
                get(product_action_receipt),
            )
            .layer(Extension(auth.clone()))
            .with_state(state(repo.clone()));
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!(
                        "/v1/tenants/{tenant}/products/actions/{}",
                        RowId(operation).to_uuid()
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
}

#[tokio::test]
async fn product_find_rejects_foreign_tenant_denied_permission_and_invalid_request() {
    let tenant = TenantId(uuid::Uuid::from_u128(31));
    for (path_tenant, permissions, text, expected) in [
        (
            TenantId(uuid::Uuid::from_u128(99)),
            vec!["skus.read".to_string()],
            "needle",
            StatusCode::FORBIDDEN,
        ),
        (tenant, vec![], "needle", StatusCode::FORBIDDEN),
        (
            tenant,
            vec!["skus.read".to_string()],
            "",
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let auth = AuthContext {
            tenant_id: tenant,
            user_id: UserId(uuid::Uuid::from_u128(32)),
            perms: normalize_permissions(permissions),
            is_admin: false,
            shop_ids: vec![],
            warehouse_ids: vec![],
            merchant_ids: vec![],
            organization_ids: vec![],
        };
        let app = Router::new()
            .route(
                "/v1/tenants/:tenant_id/products/find",
                post(find_product_cell),
            )
            .layer(Extension(auth))
            .with_state(state(Arc::new(Repo::new())));
        let response=app.oneshot(json_request(&format!("/v1/tenants/{path_tenant}/products/find"),serde_json::json!({"query":query_body(),"text":text,"columns":[0,1],"case_sensitive":false,"backward":false}))).await.unwrap();
        assert_eq!(response.status(), expected);
    }
}
