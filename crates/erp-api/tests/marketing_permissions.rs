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

#[tokio::test]
async fn marketing_price_lists_forbidden_without_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec![]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/marketing/price-lists"))
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
async fn marketing_promotions_create_forbidden_without_write_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["marketing_promotions.read"]);

    let payload = serde_json::json!({
        "name": "test-promotion",
        "promo_type": "FULL_REDUCTION"
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/marketing/promotions"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("error response should be json");
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("forbidden"));
}

#[tokio::test]
async fn marketing_coupons_create_forbidden_without_write_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["marketing_coupons.read"]);

    let payload = serde_json::json!({
        "name": "test-coupon",
        "discount_type": "PERCENTAGE",
        "discount_value": 9
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/marketing/coupons"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("error response should be json");
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("forbidden"));
}

#[tokio::test]
async fn marketing_tenant_mismatch_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();
    let token = build_token(&jwt, &tenant_b, vec!["marketing_prices.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/marketing/price-lists"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn marketing_costs_tenant_mismatch_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();
    let token = build_token(&jwt, &tenant_b, vec!["marketing_costs.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/marketing/costs"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn marketing_postmortems_tenant_mismatch_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();
    let token = build_token(&jwt, &tenant_b, vec!["marketing_postmortems.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/marketing/postmortems"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
