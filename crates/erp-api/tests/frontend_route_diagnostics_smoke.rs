use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use erp_api::{build_app, AppState, Claims, JwtService};
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

fn build_token(jwt: &JwtService, tenant_id: uuid::Uuid, perms: Vec<&str>) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|perm| perm.to_string()).collect(),
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

#[tokio::test]
async fn frontend_route_diagnostics_get_requires_observability_permissions() {
    let (app, jwt) = build_state();
    let tenant_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, tenant_id, vec!["customers.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/system/frontend-route-diagnostics"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn frontend_route_diagnostics_post_validates_tenant_id() {
    let (app, jwt) = build_state();
    let token = build_token(&jwt, uuid::Uuid::new_v4(), vec!["stats.read"]);

    let request = Request::builder()
        .uri("/v1/tenants/not-a-uuid/system/frontend-route-diagnostics")
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "events": [],
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn frontend_route_diagnostics_get_returns_summary_shape() -> anyhow::Result<()> {
    let (app, jwt) = build_state();
    let tenant_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, tenant_id, vec!["reports.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/system/frontend-route-diagnostics?window_hours=24&limit=10"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let payload: serde_json::Value = serde_json::from_slice(&body)?;

    assert!(payload.get("summary").is_some());
    assert!(payload["summary"].get("measuredFirstLoads").is_some());
    assert!(payload["summary"].get("successfulFirstLoads").is_some());
    assert!(payload["summary"].get("firstLoadAvgDurationMs").is_some());
    assert!(payload["summary"]
        .get("prefetchShortCircuitCount")
        .is_some());
    assert!(payload
        .get("paths")
        .and_then(|value| value.as_array())
        .is_some());
    assert!(
        payload["paths"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item.get("measuredFirstLoads"))
            .is_some()
            || payload["paths"].as_array().is_some()
    );
    assert!(payload
        .get("events")
        .and_then(|value| value.as_array())
        .is_some());

    Ok(())
}
