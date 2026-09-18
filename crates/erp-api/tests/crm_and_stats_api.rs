use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
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

fn build_json_request(
    uri: String,
    method: &str,
    token: &str,
    body: serde_json::Value,
) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method(method)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn crm_customer_segments_get_requires_customers_read() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/crm/customer-segments"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn crm_customer_segments_post_validates_name() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = build_json_request(
        format!("/v1/tenants/{tenant}/crm/customer-segments"),
        "POST",
        &token,
        serde_json::json!({
            "name": "   ",
            "segment_type": "DYNAMIC"
        }),
    );
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_customer_segments_put_requires_fields() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let segment_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = build_json_request(
        format!("/v1/tenants/{tenant}/crm/customer-segments/{segment_id}"),
        "PUT",
        &token,
        serde_json::json!({}),
    );
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_customer_segments_delete_rejects_bad_segment_id() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/crm/customer-segments/not-a-uuid"
        ))
        .method("DELETE")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_lifecycle_get_requires_customers_read() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/crm/lifecycle-tasks"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn crm_lifecycle_post_validates_stage() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = build_json_request(
        format!("/v1/tenants/{tenant}/crm/lifecycle-tasks"),
        "POST",
        &token,
        serde_json::json!({
            "stage": "INVALID",
            "strategy": "触达策略"
        }),
    );
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_lifecycle_put_requires_fields() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let task_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = build_json_request(
        format!("/v1/tenants/{tenant}/crm/lifecycle-tasks/{task_id}"),
        "PUT",
        &token,
        serde_json::json!({}),
    );
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_lifecycle_delete_rejects_bad_task_id() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/crm/lifecycle-tasks/not-a-uuid"
        ))
        .method("DELETE")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_service_tickets_get_validates_updated_from() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/crm/service-tickets?updated_from=bad-time"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_service_tickets_post_requires_customer_name() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = build_json_request(
        format!("/v1/tenants/{tenant}/crm/service-tickets"),
        "POST",
        &token,
        serde_json::json!({
            "customer_name": "   ",
            "priority": "HIGH"
        }),
    );
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_service_tickets_put_requires_fields() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let ticket_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = build_json_request(
        format!("/v1/tenants/{tenant}/crm/service-tickets/{ticket_id}"),
        "PUT",
        &token,
        serde_json::json!({}),
    );
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_service_tickets_delete_rejects_bad_ticket_id() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.write"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/crm/service-tickets/not-a-uuid"
        ))
        .method("DELETE")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_points_ledgers_get_validates_time_range() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/crm/points-ledgers?created_from=not-a-date"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn crm_stored_value_ledgers_get_validates_time_range() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/crm/stored-value-ledgers?created_to=not-a-date"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn stats_get_requires_stats_or_reports_permission() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["customers.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/stats/sales?start_at=2026-03-01T00:00:00Z&end_at=2026-03-05T23:59:59Z"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn stats_get_validates_stats_type() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let token = build_token(&jwt, &tenant, vec!["stats.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant}/stats/unknown?start_at=2026-03-01T00:00:00Z&end_at=2026-03-05T23:59:59Z"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
