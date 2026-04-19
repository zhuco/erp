use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use erp_api::{app_state::AppState, router::build_app as build_app_from_module, JwtService};
use erp_repo::db::Db;
use tower::ServiceExt;

mod support;

#[tokio::test]
async fn modular_exports_build_healthz_app() {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app_from_module(AppState::new(db, std::sync::Arc::new(jwt)));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("app should respond");

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("healthz body should be readable");
    match status {
        StatusCode::OK => assert_eq!(body.as_ref(), b"ok"),
        StatusCode::SERVICE_UNAVAILABLE => assert_eq!(body.as_ref(), b"db not ready"),
        status => panic!("unexpected healthz status: {status}"),
    }
}
