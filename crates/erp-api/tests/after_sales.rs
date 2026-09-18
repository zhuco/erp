use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use tower::ServiceExt;

mod support;

fn build_state() -> (axum::Router, JwtService) {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::new(db, std::sync::Arc::new(jwt.clone()));
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
async fn after_sales_create_forbidden_without_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();

    let token = build_token(&jwt, &tenant, vec!["aftersales.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/after-sales"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "order_id": uuid::Uuid::new_v4().to_string(),
                "type": "REFUND_ONLY",
                "items": [{
                    "sku_id": uuid::Uuid::new_v4().to_string(),
                    "qty": 1,
                    "refund_cent": 1000
                }]
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("error response should be json");
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("forbidden"));
}

#[tokio::test]
async fn after_sales_refund_forbidden_without_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let after_sales_id = uuid::Uuid::new_v4();

    let token = build_token(&jwt, &tenant, vec!["aftersales.read"]);
    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/after-sales/{after_sales_id}/refund"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "amount_cent": 1000,
                "channel": "MANUAL"
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn after_sales_receive_forbidden_without_inventory_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let after_sales_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();

    let token = build_token(&jwt, &tenant, vec!["aftersales.write"]);
    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/after-sales/{after_sales_id}/receive"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "warehouse_id": warehouse_id,
                "putaway_done": true
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn after_sales_putaway_forbidden_without_inventory_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let after_sales_id = uuid::Uuid::new_v4();

    let token = build_token(&jwt, &tenant, vec!["aftersales.write"]);
    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/after-sales/{after_sales_id}/putaway"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn after_sales_receive_cancel_forbidden_without_inventory_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let after_sales_id = uuid::Uuid::new_v4();

    let token = build_token(&jwt, &tenant, vec!["aftersales.write"]);
    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/after-sales/{after_sales_id}/receive/cancel"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn after_sales_receive_redo_forbidden_without_inventory_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let after_sales_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();

    let token = build_token(&jwt, &tenant, vec!["aftersales.write"]);
    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/after-sales/{after_sales_id}/receive/redo"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "warehouse_id": warehouse_id,
                "putaway_done": true
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}
