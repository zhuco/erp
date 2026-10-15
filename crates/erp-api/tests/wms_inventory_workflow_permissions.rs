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
async fn procurement_plan_generate_forbidden_without_write_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["procurement_plans.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/procurement-plans/generate"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "max_items": 10
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn inbound_settings_read_forbidden_without_inbound_read_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["settings.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/wms/inbound-settings"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn inbound_settings_write_forbidden_without_inbound_write_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["wms.inbounds.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/wms/inbound-settings"))
        .method("PUT")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "require_audit": true
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn inout_records_forbidden_without_inventory_read_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["orders.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/wms/inout-records"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn stock_change_details_forbidden_without_inventory_read_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["orders.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/wms/stock-change-details"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn stock_count_audit_forbidden_without_inventory_write_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let count_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["inventory.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/stock-counts/{count_id}/audit"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn stock_transfer_outbound_alias_forbidden_without_inventory_write_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let transfer_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["inventory.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/stock-transfers/{transfer_id}/outbound"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn stock_transfer_inbound_alias_forbidden_without_inventory_write_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let transfer_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["inventory.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/stock-transfers/{transfer_id}/inbound"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn purchase_order_inbound_alias_forbidden_without_purchase_write_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let po_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["purchase_orders.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/purchase-orders/{po_id}/inbound"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "warehouse_id": uuid::Uuid::new_v4(),
                "items": []
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn inbound_settlement_list_forbidden_without_reconciliation_read_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["procurement_reconciliations.write"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/wms/inbound-settlements"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}

#[tokio::test]
async fn inbound_settlement_create_forbidden_without_reconciliation_write_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["procurement_reconciliations.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/wms/inbound-settlements"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "supplier_id": uuid::Uuid::new_v4(),
                "period_start": "2026-04-01",
                "period_end": "2026-04-07",
                "amount_cent": 10000,
                "status": "OPEN",
                "remark": "inbound settlement"
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_forbidden(response).await;
}
