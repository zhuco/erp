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
    let claims = Claims::bearer(
        uuid::Uuid::new_v4().to_string(),
        tenant_id.to_string(),
        perms.into_iter().map(|p| p.to_string()).collect(),
        false,
        now,
        now + 3600,
    );
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
async fn platform_order_manual_download_forbidden_without_orders_download() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let shop_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["orders.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/shops/{shop_id}/platform-orders/download/manual"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "cursor": "next-cursor"
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn platform_order_time_range_download_forbidden_without_orders_download() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let shop_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["orders.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/shops/{shop_id}/platform-orders/download/by-time-range"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "start_time": "2026-01-01T00:00:00Z",
                "end_time": "2026-01-01T01:00:00Z"
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn shop_product_parse_forbidden_without_shop_product_permissions() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["orders.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/shop-products/parse"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "sku_record_ids": [uuid::Uuid::new_v4().to_string()]
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn bundle_auto_match_forbidden_without_shop_product_write() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["shop_products.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/shop-products/bundle-match/auto"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "sku_record_ids": [uuid::Uuid::new_v4().to_string()]
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn bundle_manual_match_forbidden_without_shop_product_write() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["shop_products.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/shop-products/bundle-match/manual"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "sku_record_ids": [uuid::Uuid::new_v4().to_string()],
                "sku_id": uuid::Uuid::new_v4().to_string()
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn update_order_sync_settings_forbidden_without_required_permissions() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let shop_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["orders.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/shops/{shop_id}/sync-settings/orders"
        ))
        .method("PUT")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "auto_download_orders": true
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn platform_inventory_sync_run_forbidden_without_inventory_write() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let shop_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["inventory.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/shops/{shop_id}/platform-sync/inventory/run"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn platform_after_sales_sync_run_forbidden_without_aftersales_write() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let shop_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["aftersales.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/shops/{shop_id}/platform-sync/after-sales/run"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}
