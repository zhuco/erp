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
async fn waiting_ship_queue_forbidden_without_waiting_ship_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["wms.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/wms/outbounds/waiting-ship"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn wave_ship_forbidden_without_wave_ship_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let wave_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["wms.write"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/wms/waves/{wave_id}/ship"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "continue_on_error": true
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn direct_ship_forbidden_without_direct_ship_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let outbound_doc_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["wms.write"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/wms/outbounds/{outbound_doc_id}/direct-ship"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "reason": "紧急直接发货"
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn operator_register_forbidden_without_operator_register_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let outbound_doc_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["wms.write"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/wms/outbounds/{outbound_doc_id}/operator-register"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "picker_user_id": uuid::Uuid::new_v4(),
                "packer_user_id": uuid::Uuid::new_v4(),
                "note": "晚班补登记"
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}
