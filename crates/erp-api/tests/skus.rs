use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, ChannelSkuMappingRepo, Claims, JwtService, SkuRepo, build_app};
use erp_repo::{
    channel_sku_mappings::{
        ChannelSkuMapping, ChannelSkuMappingListFilters, ChannelSkuMappingListResult,
    },
    db::Db,
    skus::{
        Sku, SkuCreate, SkuListFilters, SkuListResult, SkuSearchFilters, SkuSearchItem, SkuUpdate,
    },
};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod support;

struct SkusRepoStub;

#[async_trait::async_trait]
impl SkuRepo for SkusRepoStub {
    async fn execute_grid_action(
        &self,
        _tenant: erp_types::TenantId,
        _actor: erp_types::UserId,
        _command: &erp_grid_core::actions::ActionCommand,
    ) -> Result<erp_grid_core::actions::ActionReceipt, erp_repo::product_actions::ActionError> {
        Err(erp_repo::product_actions::ActionError::Invalid(
            "not implemented",
        ))
    }

    async fn grid_action_receipt(
        &self,
        _tenant: erp_types::TenantId,
        _actor: erp_types::UserId,
        _operation: erp_grid_core::RowId,
    ) -> Result<Option<erp_grid_core::actions::ActionReceipt>, erp_repo::product_actions::ActionError>
    {
        Ok(None)
    }

    async fn query_skus(
        &self,
        _tenant_id: erp_types::TenantId,
        _product_id: uuid::Uuid,
        _query: &erp_grid_core::paged_query::PageQuery,
    ) -> anyhow::Result<erp_repo::sku_grid::SkuGridPage> {
        Ok(erp_repo::sku_grid::SkuGridPage {
            data: Vec::new(),
            total: 0,
        })
    }

    async fn list_skus(
        &self,
        _tenant_id: erp_types::TenantId,
        _filters: SkuListFilters,
    ) -> anyhow::Result<SkuListResult> {
        Ok(SkuListResult {
            items: Vec::new(),
            next_cursor: None,
        })
    }

    async fn search_skus(
        &self,
        _tenant_id: erp_types::TenantId,
        _filters: SkuSearchFilters,
    ) -> anyhow::Result<Vec<SkuSearchItem>> {
        Ok(Vec::new())
    }

    async fn get_sku(
        &self,
        _tenant_id: erp_types::TenantId,
        _sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Sku>> {
        Ok(None)
    }

    async fn create_sku(&self, _input: SkuCreate) -> anyhow::Result<Sku> {
        anyhow::bail!("not implemented")
    }

    async fn update_sku(
        &self,
        _tenant_id: erp_types::TenantId,
        _sku_id: uuid::Uuid,
        _input: SkuUpdate,
    ) -> anyhow::Result<Option<Sku>> {
        anyhow::bail!("not implemented")
    }

    async fn delete_sku(
        &self,
        _tenant_id: erp_types::TenantId,
        _sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Sku>> {
        Ok(None)
    }
}

struct ChannelSkuMappingsRepoStub;

#[async_trait::async_trait]
impl ChannelSkuMappingRepo for ChannelSkuMappingsRepoStub {
    async fn list_mappings(
        &self,
        _tenant_id: erp_types::TenantId,
        _filters: ChannelSkuMappingListFilters,
    ) -> anyhow::Result<ChannelSkuMappingListResult> {
        Ok(ChannelSkuMappingListResult {
            items: Vec::new(),
            next_cursor: None,
        })
    }

    async fn create_mapping(
        &self,
        _input: erp_repo::channel_sku_mappings::ChannelSkuMappingCreate,
    ) -> anyhow::Result<ChannelSkuMapping> {
        anyhow::bail!("not implemented")
    }

    async fn update_mapping(
        &self,
        _tenant_id: erp_types::TenantId,
        _mapping_id: uuid::Uuid,
        _input: erp_repo::channel_sku_mappings::ChannelSkuMappingUpdate,
    ) -> anyhow::Result<Option<ChannelSkuMapping>> {
        anyhow::bail!("not implemented")
    }
}

fn build_state() -> (axum::Router, JwtService) {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::with_sku_repos(
        db,
        std::sync::Arc::new(jwt.clone()),
        std::sync::Arc::new(SkusRepoStub),
        std::sync::Arc::new(ChannelSkuMappingsRepoStub),
    );
    let app = build_app(state);
    (app, jwt)
}

fn build_token(jwt: &JwtService, tenant_id: &str, perms: Vec<&str>, is_admin: bool) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims::bearer(
        uuid::Uuid::new_v4().to_string(),
        tenant_id.to_string(),
        perms.into_iter().map(|p| p.to_string()).collect(),
        is_admin,
        now,
        now + 3600,
    );
    jwt.encode(&claims).expect("token should encode")
}

fn tenant_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[tokio::test]
async fn skus_list_forbidden_without_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();

    let token = build_token(&jwt, &tenant, vec![], false);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/skus"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("error response should be json");
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("forbidden"));
}

#[tokio::test]
async fn skus_list_ok_shape() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();

    let token = build_token(&jwt, &tenant, vec!["skus.read"], false);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/skus"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("list response should be json");
    assert!(json.get("data").map(|v| v.is_array()).unwrap_or(false));
    assert!(json.get("next_cursor").is_some());
}
