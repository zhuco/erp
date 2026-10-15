//! Original reporting outcomes are read under current business authority.
use super::*;
use erp_repo::shared_contract::{OperationReportError, ReceiptValueAuthorizationError};

pub(super) fn failure(error: anyhow::Error) -> Response {
    let (status, code) = if let Some(auth) = error.downcast_ref::<ReceiptValueAuthorizationError>()
    {
        match auth {
            ReceiptValueAuthorizationError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "report_session_required")
            }
            ReceiptValueAuthorizationError::Forbidden => {
                (StatusCode::FORBIDDEN, "report_permission_required")
            }
            ReceiptValueAuthorizationError::Unavailable => {
                (StatusCode::NOT_FOUND, "report_not_found")
            }
        }
    } else if let Some(report) = error.downcast_ref::<OperationReportError>() {
        match report {
            OperationReportError::ContextLimit => {
                (StatusCode::UNPROCESSABLE_ENTITY, "report_context_limit")
            }
            OperationReportError::NotFound => (StatusCode::NOT_FOUND, "report_not_found"),
            OperationReportError::SessionMismatch => {
                (StatusCode::FORBIDDEN, "report_session_mismatch")
            }
            OperationReportError::PayloadConflict => {
                (StatusCode::CONFLICT, "report_payload_conflict")
            }
            OperationReportError::ReconciliationRequired => {
                (StatusCode::CONFLICT, "report_reconciliation_required")
            }
            OperationReportError::Rejected => (StatusCode::UNPROCESSABLE_ENTITY, "report_rejected"),
        }
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "report_storage_unavailable",
        )
    };
    (status, Json(serde_json::json!({"code":code}))).into_response()
}

pub(super) async fn read(
    State(state): State<AppState>,
    Path((path, order, request)): Path<(String, uuid::Uuid, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(t) => t,
        Err(r) => return r,
    };
    let actor = match work_order_catalog::actor(&ctx, epoch) {
        Ok(a) => a,
        Err(r) => return r,
    };
    if request.is_empty() || request.len() > 128 || request.chars().any(char::is_control) {
        return bad_request("bad report request_id").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_authorized_operation_report(tenant, order, &request, actor)
        .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(error) => failure(error),
    }
}

pub(super) async fn context(
    State(state): State<AppState>,
    Path((path, order)): Path<(String, uuid::Uuid)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(t) => t,
        Err(r) => return r,
    };
    let actor = match work_order_catalog::actor(&ctx, epoch) {
        Ok(a) => a,
        Err(r) => return r,
    };
    match SharedContractRepo::new(state.db.pool())
        .read_reporting_context(tenant, order, actor)
        .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(error) => failure(error),
    }
}
