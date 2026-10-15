use super::*;
pub(super) fn error(error: anyhow::Error) -> Response {
    if let Some(auth) =
        error.downcast_ref::<erp_repo::shared_contract::ReceiptValueAuthorizationError>()
    {
        let status = match auth {
            erp_repo::shared_contract::ReceiptValueAuthorizationError::Unauthorized => {
                StatusCode::UNAUTHORIZED
            }
            erp_repo::shared_contract::ReceiptValueAuthorizationError::Forbidden => {
                StatusCode::FORBIDDEN
            }
            erp_repo::shared_contract::ReceiptValueAuthorizationError::Unavailable => {
                StatusCode::NOT_FOUND
            }
        };
        return (status, error.to_string()).into_response();
    }
    if error.downcast_ref::<sqlx::Error>().is_some() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "production cost storage unavailable",
        )
            .into_response();
    }
    if matches!(
        error.downcast_ref::<erp_repo::shared_contract::MaterialCostError>(),
        Some(erp_repo::shared_contract::MaterialCostError::Reconcile(_))
    ) {
        return (StatusCode::CONFLICT,Json(serde_json::json!({"code":"material_cost_reconcile_required","reconciliation_required":true,"message":error.to_string()}))).into_response();
    }
    (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()).into_response()
}
