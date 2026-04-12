use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{build_app, AppState, Claims, JwtService};
use erp_repo::db::Db;
use tower::ServiceExt;

mod support;

fn build_state() -> (axum::Router, JwtService) {
    std::env::set_var("IDEMPOTENCY_DISABLED", "true");
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

#[tokio::test]
async fn tenant_mismatch_orders_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();
    let order_id = uuid::Uuid::new_v4();

    let token = build_token(&jwt, &tenant_b, vec!["orders.read", "orders.read_raw"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/orders/{order_id}/raw"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tenant_mismatch_inventory_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();

    let token = build_token(&jwt, &tenant_b, vec!["inventory.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/inventory"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tenant_mismatch_shipping_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();

    let token = build_token(&jwt, &tenant_b, vec!["orders.write", "shipping.write"]);
    let body = serde_json::json!({
        "order_id": uuid::Uuid::new_v4(),
        "warehouse_id": uuid::Uuid::new_v4(),
        "carrier_code": "ZTO",
        "tracking_no": "TRACK-403"
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/shipments"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", "tenant-mismatch-ship")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tenant_mismatch_printing_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();

    let token = build_token(&jwt, &tenant_b, vec!["print_jobs.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/print-jobs"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tenant_mismatch_aftersales_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();

    let token = build_token(&jwt, &tenant_b, vec!["aftersales.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/after-sale-cases"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
