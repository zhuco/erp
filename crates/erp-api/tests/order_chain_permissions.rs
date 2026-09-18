use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;

mod support;

fn build_state() -> (Router, JwtService) {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::new(db, Arc::new(jwt.clone()));
    let app = build_app(state);
    (app, jwt)
}

fn build_token(jwt: &JwtService, tenant_id: &str, perms: Vec<&str>) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|p| p.to_string()).collect(),
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

fn tenant_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

async fn assert_forbidden(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("error response should be json");
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("forbidden"));
}

#[tokio::test]
async fn order_chain_forbidden_without_shipping_read() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let order_id = uuid::Uuid::new_v4();
    let token = build_token(
        &jwt,
        &tenant,
        vec!["orders.read", "aftersales.read", "wms.inbounds.read"],
    );

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/orders/{order_id}/chain"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn oms_order_chain_forbidden_without_aftersales_read() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let order_id = uuid::Uuid::new_v4();
    let token = build_token(
        &jwt,
        &tenant,
        vec!["orders.read", "shipping.read", "wms.inbounds.read"],
    );

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/oms/orders/{order_id}/chain"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn order_chain_forbidden_without_inbounds_read() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let order_id = uuid::Uuid::new_v4();
    let token = build_token(
        &jwt,
        &tenant,
        vec!["orders.read", "shipping.read", "aftersales.read"],
    );

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/orders/{order_id}/chain"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}
