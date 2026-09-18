use axum::{
    Json,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use erp_repo::idempotency::{IdempotencyKey, IdempotencyKeyInsert, IdempotencyRepo};
use erp_types::TenantId;
use http_body_util::BodyExt;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use tracing::warn;

use crate::AppState;

#[derive(Serialize)]
struct ErrorResponse {
    code: String,
    message: String,
    details: Option<Value>,
    request_id: Option<String>,
}

pub async fn idempotency_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if idempotency_disabled() {
        return next.run(req).await;
    }
    if !is_idempotent_method(req.method()) {
        return next.run(req).await;
    }

    let path = req.uri().path().to_string();
    let Some(tenant_id) = match_idempotency_tenant(&path) else {
        return next.run(req).await;
    };

    let idempotency_key = match req
        .headers()
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => value.to_string(),
        None => return next.run(req).await,
    };

    let (parts, body) = req.into_parts();
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            warn!(error = %err, "idempotency body read failed");
            return (StatusCode::BAD_REQUEST, "invalid body").into_response();
        }
    };

    let request_hash = build_request_hash(tenant_id, &path, &body_bytes);
    let repo = IdempotencyRepo::new(state.db.pool());
    match repo.find_key(tenant_id, &path, &idempotency_key).await {
        Ok(Some(record)) => {
            if record.request_hash == request_hash {
                return cached_response(record);
            }
            return conflict_response();
        }
        Ok(None) => {}
        Err(err) => {
            warn!(error = %err, "idempotency lookup failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "idempotency lookup failed",
            )
                .into_response();
        }
    }

    let req = Request::from_parts(parts, Body::from(body_bytes.clone()));
    let response = next.run(req).await;
    let (parts, body) = response.into_parts();
    let response_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            warn!(error = %err, "idempotency response read failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, "response read failed").into_response();
        }
    };

    let response_body = parse_response_body(&response_bytes);
    let response_status = i32::from(parts.status.as_u16());
    if let Err(err) = repo
        .insert_key(IdempotencyKeyInsert {
            tenant_id,
            key: idempotency_key,
            route: path,
            request_hash,
            response_status,
            response_body: response_body.clone(),
        })
        .await
    {
        warn!(error = %err, "idempotency insert failed");
    }

    Response::from_parts(parts, Body::from(response_bytes))
}

fn match_idempotency_tenant(path: &str) -> Option<TenantId> {
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    if segments.len() < 4 {
        return None;
    }
    if segments[0] != "v1" || segments[1] != "tenants" {
        return None;
    }

    let tenant_id = TenantId::from_str(segments[2]).ok()?;
    match segments.as_slice() {
        ["v1", "tenants", _, "oms", "orders", _, "allocate"]
        | ["v1", "tenants", _, "oms", "orders", _, "submit-shipment"]
        | [
            "v1",
            "tenants",
            _,
            "oms",
            "orders",
            "recheck-queue",
            "batch-review",
        ]
        | ["v1", "tenants", _, "oms", "orders", _, "shipments"]
        | ["v1", "tenants", _, "oms", "orders", _, "resend"]
        | ["v1", "tenants", _, "oms", "orders", _, "release"]
        | ["v1", "tenants", _, "oms", "review-settings"]
        | ["v1", "tenants", _, "oms", "shipments", _, "ship"]
        | ["v1", "tenants", _, "oms", "shipments", _, "cancel-request"]
        | ["v1", "tenants", _, "orders", _, "ship"]
        | ["v1", "tenants", _, "shipments"]
        | ["v1", "tenants", _, "after-sale-cases"]
        | ["v1", "tenants", _, "after-sales"]
        | ["v1", "tenants", _, "after-sales", "import"]
        | ["v1", "tenants", _, "after-sales", _, "approve"]
        | ["v1", "tenants", _, "after-sales", _, "reject"]
        | ["v1", "tenants", _, "after-sales", _, "exchange"]
        | ["v1", "tenants", _, "after-sales", _, "resend"]
        | ["v1", "tenants", _, "after-sales", _, "receive"]
        | ["v1", "tenants", _, "after-sales", _, "refund"]
        | ["v1", "tenants", _, "inventory", "adjustments"]
        | ["v1", "tenants", _, "inventory", "inbounds"]
        | ["v1", "tenants", _, "inventory", "outbounds"]
        | ["v1", "tenants", _, "inventory", "reservations"]
        | ["v1", "tenants", _, "inventory", "releases"]
        | ["v1", "tenants", _, "inventory", "freezes"]
        | ["v1", "tenants", _, "inventory", "freezes", _, "cancel"]
        | ["v1", "tenants", _, "stock-counts"]
        | ["v1", "tenants", _, "stock-counts", "import"]
        | ["v1", "tenants", _, "stock-counts", _, "items"]
        | ["v1", "tenants", _, "stock-counts", _, "confirm-diff"]
        | ["v1", "tenants", _, "stock-counts", _, "apply"]
        | ["v1", "tenants", _, "stock-counts", _, "cancel"]
        | ["v1", "tenants", _, "marketing", "price-lists"]
        | ["v1", "tenants", _, "marketing", "price-lists", _, "items"]
        | ["v1", "tenants", _, "marketing", "promotions"]
        | ["v1", "tenants", _, "marketing", "gift-policies"]
        | ["v1", "tenants", _, "marketing", "coupons"]
        | ["v1", "tenants", _, "marketing", "costs"]
        | ["v1", "tenants", _, "marketing", "postmortems"]
        | ["v1", "tenants", _, "stock-transfers"]
        | ["v1", "tenants", _, "stock-transfers", _, "confirm-outbound"]
        | ["v1", "tenants", _, "stock-transfers", _, "confirm-inbound"]
        | ["v1", "tenants", _, "stock-adjustments"]
        | ["v1", "tenants", _, "stock-adjustments", _, "apply"]
        | ["v1", "tenants", _, "stock-moves"]
        | ["v1", "tenants", _, "stock-moves", _, "apply"]
        | ["v1", "tenants", _, "inv", "adjust"]
        | ["v1", "tenants", _, "inv", "sync-jobs", _, "retry"]
        | ["v1", "tenants", _, "inv", "sync-jobs", _, "run"]
        | ["v1", "tenants", _, "wms", "inbounds"]
        | ["v1", "tenants", _, "wms", "inbounds", _, "receive"]
        | ["v1", "tenants", _, "wms", "inbounds", _, "putaway"]
        | ["v1", "tenants", _, "wms", "outbounds"]
        | ["v1", "tenants", _, "wms", "outbounds", _, "pack"]
        | ["v1", "tenants", _, "wms", "outbounds", _, "ship"]
        | ["v1", "tenants", _, "wms", "outbounds", _, "verify", "start"]
        | [
            "v1",
            "tenants",
            _,
            "wms",
            "outbounds",
            _,
            "verify",
            "scan-item",
        ]
        | [
            "v1",
            "tenants",
            _,
            "wms",
            "outbounds",
            _,
            "verify",
            "finish",
        ]
        | ["v1", "tenants", _, "wms", "waves"]
        | ["v1", "tenants", _, "wms", "pick-tasks", _, "confirm"]
        | ["v1", "tenants", _, "wms", "labor", "tasks", _, "assign"]
        | ["v1", "tenants", _, "wms", "labor", "tasks", _, "start"]
        | ["v1", "tenants", _, "wms", "labor", "tasks", _, "complete"]
        | [
            "v1",
            "tenants",
            _,
            "wms",
            "slotting",
            "suggestions",
            _,
            "apply",
        ]
        | [
            "v1",
            "tenants",
            _,
            "wms",
            "slotting",
            "suggestions",
            _,
            "reject",
        ]
        | [
            "v1",
            "tenants",
            _,
            "wms",
            "replenishment",
            "tasks",
            _,
            "claim",
        ]
        | [
            "v1",
            "tenants",
            _,
            "wms",
            "replenishment",
            "tasks",
            _,
            "complete",
        ]
        | [
            "v1",
            "tenants",
            _,
            "wms",
            "dock",
            "appointments",
            _,
            "check-in",
        ]
        | [
            "v1",
            "tenants",
            _,
            "wms",
            "dock",
            "appointments",
            _,
            "start-loading",
        ]
        | [
            "v1",
            "tenants",
            _,
            "wms",
            "dock",
            "appointments",
            _,
            "depart",
        ]
        | ["v1", "tenants", _, "purchase-orders"]
        | ["v1", "tenants", _, "purchase-orders", _, "send"]
        | ["v1", "tenants", _, "purchase-orders", _, "receive"]
        | ["v1", "tenants", _, "purchase-returns"]
        | ["v1", "tenants", _, "purchase-returns", _, "confirm"]
        | ["v1", "tenants", _, "fin", "statements", "import"]
        | ["v1", "tenants", _, "fin", "reconcile", "run"]
        | ["v1", "tenants", _, "fin", "invoices", "apply"]
        | ["v1", "tenants", _, "print-jobs"] => Some(tenant_id),
        _ => None,
    }
}

fn build_request_hash(tenant_id: TenantId, route: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tenant_id.to_string());
    hasher.update(b"|");
    hasher.update(route);
    hasher.update(b"|");
    hasher.update(body);
    hex::encode(hasher.finalize())
}

fn parse_response_body(body: &[u8]) -> Value {
    if body.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(body).to_string()))
}

fn is_idempotent_method(method: &Method) -> bool {
    matches!(*method, Method::POST | Method::PUT)
}

#[cfg(test)]
mod tests {
    use super::match_idempotency_tenant;

    #[test]
    fn matches_new_wms_execution_action_paths() {
        let tenant_id = uuid::Uuid::new_v4();
        let tenant = tenant_id.to_string();
        let should_match = [
            format!("/v1/tenants/{tenant}/wms/labor/tasks/LAB-1/assign"),
            format!("/v1/tenants/{tenant}/wms/labor/tasks/LAB-1/start"),
            format!("/v1/tenants/{tenant}/wms/labor/tasks/LAB-1/complete"),
            format!("/v1/tenants/{tenant}/wms/slotting/suggestions/SLOT-1/apply"),
            format!("/v1/tenants/{tenant}/wms/slotting/suggestions/SLOT-1/reject"),
            format!("/v1/tenants/{tenant}/wms/replenishment/tasks/REP-1/claim"),
            format!("/v1/tenants/{tenant}/wms/replenishment/tasks/REP-1/complete"),
            format!("/v1/tenants/{tenant}/wms/dock/appointments/DOCK-1/check-in"),
            format!("/v1/tenants/{tenant}/wms/dock/appointments/DOCK-1/start-loading"),
            format!("/v1/tenants/{tenant}/wms/dock/appointments/DOCK-1/depart"),
        ];
        for path in should_match {
            let parsed = match_idempotency_tenant(&path)
                .unwrap_or_else(|| panic!("path should match idempotency tenant: {path}"));
            assert_eq!(parsed.to_string(), tenant);
        }
    }

    #[test]
    fn does_not_match_new_wms_execution_query_paths() {
        let tenant = uuid::Uuid::new_v4();
        let should_not_match = [
            format!("/v1/tenants/{tenant}/wms/labor/tasks"),
            format!("/v1/tenants/{tenant}/wms/slotting/suggestions"),
            format!("/v1/tenants/{tenant}/wms/replenishment/tasks"),
            format!("/v1/tenants/{tenant}/wms/dock/appointments"),
        ];
        for path in should_not_match {
            assert!(
                match_idempotency_tenant(&path).is_none(),
                "path should not match idempotency tenant: {path}"
            );
        }
    }

    #[test]
    fn matches_order_chain_p0_paths() {
        let tenant_id = uuid::Uuid::new_v4();
        let tenant = tenant_id.to_string();
        let should_match = [
            format!("/v1/tenants/{tenant}/oms/orders/ORDER-1/submit-shipment"),
            format!("/v1/tenants/{tenant}/oms/orders/recheck-queue/batch-review"),
            format!("/v1/tenants/{tenant}/oms/review-settings"),
            format!("/v1/tenants/{tenant}/wms/outbounds/OUT-1/verify/start"),
            format!("/v1/tenants/{tenant}/wms/outbounds/OUT-1/verify/scan-item"),
            format!("/v1/tenants/{tenant}/wms/outbounds/OUT-1/verify/finish"),
        ];
        for path in should_match {
            let parsed = match_idempotency_tenant(&path)
                .unwrap_or_else(|| panic!("path should match idempotency tenant: {path}"));
            assert_eq!(parsed.to_string(), tenant);
        }
    }
}

fn cached_response(record: IdempotencyKey) -> Response {
    let status = StatusCode::from_u16(record.response_status as u16)
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    match record.response_body {
        Value::String(text) => {
            let mut response = Response::new(Body::from(text));
            *response.status_mut() = status;
            response
        }
        value => (status, Json(value)).into_response(),
    }
}

fn conflict_response() -> Response {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            code: "idempotency_conflict".to_string(),
            message: "Idempotency-Key conflict".to_string(),
            details: None,
            request_id: None,
        }),
    )
        .into_response()
}

fn idempotency_disabled() -> bool {
    let Ok(raw) = std::env::var("IDEMPOTENCY_DISABLED") else {
        return false;
    };
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}
