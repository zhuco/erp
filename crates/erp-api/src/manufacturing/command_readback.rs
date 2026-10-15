//! Machine-readable failures for compatibility command reads, with no business execution.
use super::*;
use erp_repo::shared_contract::CommandReadbackError;

pub(super) fn error_response(error: anyhow::Error) -> Response {
    let (status, code) = match error.downcast_ref::<CommandReadbackError>() {
        Some(CommandReadbackError::NotFound) => (StatusCode::NOT_FOUND, "command_not_found"),
        Some(CommandReadbackError::ClientTransitionDenied) => {
            (StatusCode::CONFLICT, "command_client_transition_denied")
        }
        Some(CommandReadbackError::InvalidStatus) => {
            (StatusCode::BAD_REQUEST, "command_invalid_status")
        }
        Some(CommandReadbackError::ReconciliationRequired(_)) => {
            (StatusCode::CONFLICT, "command_reconciliation_required")
        }
        Some(CommandReadbackError::SessionForbidden) => {
            (StatusCode::FORBIDDEN, "command_session_forbidden")
        }
        None => (StatusCode::UNPROCESSABLE_ENTITY, "command_readback_failed"),
    };
    (
        status,
        Json(serde_json::json!({"code":code,"message":error.to_string()})),
    )
        .into_response()
}
