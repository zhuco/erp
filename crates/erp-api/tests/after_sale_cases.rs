use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{build_app, AfterSaleRepo, AppState, Claims, JwtService};
use erp_domain::{AfterSaleCase, AfterSaleCaseDetail};
use erp_repo::after_sale_cases::AfterSaleCaseListFilters;
use erp_repo::db::Db;
use http_body_util::BodyExt;
use tower::util::ServiceExt;

mod support;

struct AfterSaleRepoStub;

#[async_trait::async_trait]
impl AfterSaleRepo for AfterSaleRepoStub {
    async fn list_cases(
        &self,
        _tenant_id: erp_types::TenantId,
        _filters: AfterSaleCaseListFilters,
    ) -> anyhow::Result<Vec<AfterSaleCase>> {
        Ok(Vec::new())
    }

    async fn get_case_detail(
        &self,
        _tenant_id: erp_types::TenantId,
        _case_id: erp_types::AfterSaleCaseId,
    ) -> anyhow::Result<Option<AfterSaleCaseDetail>> {
        Ok(None)
    }

    async fn get_case_raw(
        &self,
        _tenant_id: erp_types::TenantId,
        _case_id: erp_types::AfterSaleCaseId,
    ) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

fn build_state() -> (axum::Router, JwtService) {
    std::env::set_var("PDD_ONLY_MODE", "false");
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::with_after_sale_repo(
        db,
        std::sync::Arc::new(jwt.clone()),
        std::sync::Arc::new(AfterSaleRepoStub),
    );
    let app = build_app(state);
    (app, jwt)
}

fn build_token(jwt: &JwtService, tenant_id: &str, perms: Vec<&str>, is_admin: bool) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|p| p.to_string()).collect(),
        is_admin,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

fn tenant_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[tokio::test]
async fn after_sale_cases_authz() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();

    let token_missing_raw = build_token(&jwt, &tenant_a, vec!["aftersales.read"], false);
    let raw_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_a}/after-sale-cases/{}/raw",
            uuid::Uuid::new_v4()
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token_missing_raw}"))
        .body(Body::empty())
        .unwrap();
    let raw_response = app.clone().oneshot(raw_request).await.unwrap();
    assert_eq!(raw_response.status(), StatusCode::FORBIDDEN);
    let raw_body = raw_response.into_body().collect().await.unwrap().to_bytes();
    let raw_json: serde_json::Value =
        serde_json::from_slice(&raw_body).expect("error response should be json");
    assert_eq!(
        raw_json.get("code").and_then(|v| v.as_str()),
        Some("forbidden")
    );

    let token_other_tenant = build_token(&jwt, &tenant_b, vec!["aftersales.read"], false);
    let list_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/after-sale-cases"))
        .method("GET")
        .header("authorization", format!("Bearer {token_other_tenant}"))
        .body(Body::empty())
        .unwrap();
    let list_response = app.oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::FORBIDDEN);
    let list_body = list_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let list_json: serde_json::Value =
        serde_json::from_slice(&list_body).expect("error response should be json");
    assert_eq!(
        list_json.get("code").and_then(|v| v.as_str()),
        Some("forbidden")
    );
}

#[tokio::test]
async fn after_sale_cases_ok_shape() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();

    let token = build_token(&jwt, &tenant, vec!["aftersales.read"], false);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/after-sale-cases"))
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
    assert!(json.get("limit").is_some());
    assert!(json.get("offset").is_some());
}
