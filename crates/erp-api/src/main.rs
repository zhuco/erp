use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use clap::Parser;
use erp_api::{AppState, JwtService, build_app};
use erp_common::{
    settings::Settings,
    telemetry::{METRICS_CONTENT_TYPE, init_tracing, metrics, render_metrics},
};
use erp_repo::db::Db;
use secrecy::ExposeSecret;
use serde_json::Value;
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

#[derive(Parser, Debug)]
struct Args {
    /// Interface on which the API listens. Use loopback for local acceptance.
    #[arg(long, default_value = "0.0.0.0")]
    bind: std::net::IpAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing("erp-api");
    let args = Args::parse();
    let settings = Settings::load()?;

    let db = Db::connect(&settings.database_url).await?;
    let jwt = Arc::new(JwtService::new(
        settings.jwt_secret.expose_secret().to_string(),
    ));
    let nats = match async_nats::connect(settings.nats_url.clone()).await {
        Ok(client) => Some(client),
        Err(err) => {
            warn!(error = %err, "nats connect failed");
            None
        }
    };

    let state = AppState::new_with_nats(db, jwt, nats);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    let app = build_app(state)
        .route("/metrics", get(metrics_handler))
        .layer(cors)
        .layer(middleware::from_fn(metrics_middleware));

    let addr = SocketAddr::new(args.bind, settings.http_port_api);
    info!(%addr, "erp-api listening");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

async fn metrics_handler() -> Response {
    let body = render_metrics();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, METRICS_CONTENT_TYPE)
        .body(Body::from(body))
        .unwrap()
}

async fn metrics_middleware(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let start = Instant::now();
    let mut response = next.run(req).await;

    if (method == Method::PUT && path.contains("/print-jobs/"))
        || (method == Method::POST && path.contains("/print-agent/jobs/"))
    {
        response = record_print_job_metrics(response).await;
    }

    if response.status().is_success() {
        if method == Method::POST && path.ends_with("/inventory/adjustments") {
            metrics().inc_inventory_adjust();
        }
        if method == Method::POST && path.ends_with("/inventory/reservations") {
            metrics().inc_inventory_reserve();
        }
        if method == Method::POST && path.ends_with("/inventory/releases") {
            metrics().inc_inventory_return();
        }
    }

    metrics().inc_http_request_total();
    metrics().add_http_request_latency_ms(start.elapsed().as_millis() as u64);
    if response.status().is_server_error() {
        metrics().inc_http_request_error();
    }

    response
}

async fn record_print_job_metrics(response: Response) -> Response {
    if !response.status().is_success() {
        return response;
    }

    let (parts, body) = response.into_parts();
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => return Response::from_parts(parts, Body::empty()),
    };

    if let Ok(payload) = serde_json::from_slice::<Value>(&bytes) {
        if let Some(status) = payload.get("status").and_then(|v| v.as_str()) {
            if status.eq_ignore_ascii_case("DONE") {
                metrics().inc_print_jobs_success();
            } else if status.eq_ignore_ascii_case("FAILED") {
                metrics().inc_print_jobs_failed();
            }
        }
    }

    Response::from_parts(parts, Body::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn metrics_endpoint_returns_text() {
        let app = axum::Router::new().route("/metrics", get(metrics_handler));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_text = std::str::from_utf8(&body_bytes).unwrap();
        assert!(body_text.contains("# HELP"));
    }
}
