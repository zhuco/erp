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
async fn stats_health_requires_stats_or_reports_permission() {
    let (app, jwt) = build_state();
    let tenant_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, tenant_id, vec!["customers.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/stats/health"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn stats_health_requires_authentication() {
    let (app, _jwt) = build_state();
    let tenant_id = uuid::Uuid::new_v4();

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/stats/health"))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn stats_health_validates_tenant_id_format() {
    let (app, jwt) = build_state();
    let token = build_token(&jwt, uuid::Uuid::new_v4(), vec!["stats.read"]);

    let request = Request::builder()
        .uri("/v1/tenants/not-a-uuid/stats/health")
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn stats_health_allows_reports_read_permission() {
    let (app, jwt) = build_state();
    let tenant_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, tenant_id, vec!["reports.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/stats/health"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn stats_report_allows_reports_read_for_validation_path() {
    let (app, jwt) = build_state();
    let tenant_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, tenant_id, vec!["reports.read"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/stats/unknown?start_at=2026-04-01T00:00:00Z&end_at=2026-04-02T00:00:00Z"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn stats_health_returns_clickhouse_and_sso_snapshot() -> anyhow::Result<()> {
    let (app, jwt) = build_state();
    let tenant_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, tenant_id, vec!["stats.read"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/stats/health"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let payload: serde_json::Value = serde_json::from_slice(&body)?;

    assert!(payload
        .get("analytics_backend")
        .and_then(|v| v.as_str())
        .is_some());
    assert!(payload
        .get("clickhouse_ready")
        .and_then(|v| v.as_bool())
        .is_some());
    assert!(payload
        .get("clickhouse_endpoint")
        .map(|value| value.is_string() || value.is_null())
        .unwrap_or(false));
    assert!(payload.get("sso_ready").and_then(|v| v.as_bool()).is_some());
    assert!(payload
        .get("sso_issuer")
        .map(|value| value.is_string() || value.is_null())
        .unwrap_or(false));
    assert!(payload.get("checked_at").and_then(|v| v.as_str()).is_some());

    Ok(())
}
