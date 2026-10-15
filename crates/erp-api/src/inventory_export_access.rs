//! Inventory-specific authorization for the existing export job routes.
use super::*;
use erp_repo::inventory_export_jobs::{
    self, InventoryExportArtifact, InventoryExportError, InventoryExportRequest,
    InventoryExportVisibility,
};

fn access_error(error: InventoryExportError) -> Response {
    match error {
        InventoryExportError::Forbidden | InventoryExportError::Invalid(_) => error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "inventory export access unavailable; create a new request after checking permissions",
        )
        .into_response(),
        InventoryExportError::Conflict => {
            conflict("inventory export request identity conflict").into_response()
        }
        InventoryExportError::Storage(error) => {
            warn!(error=%error,"inventory export authority unavailable");
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "authorization_unavailable",
                "inventory export authorization unavailable",
            )
            .into_response()
        }
    }
}

pub(super) async fn create(state: &AppState, ctx: &AuthContext, raw: Option<Value>) -> Response {
    let params =
        match raw.and_then(|value| serde_json::from_value::<InventoryExportRequest>(value).ok()) {
            Some(params) => params,
            None => {
                return bad_request(
                    "inventory export requires request_id and query; unknown fields are rejected",
                )
                .into_response();
            }
        };
    let params = match params.normalize() {
        Ok(params) => params,
        Err(error) => return bad_request(&error.to_string()).into_response(),
    };
    match inventory_export_jobs::create_job(state.db.pool(), ctx.tenant_id, ctx.user_id, params)
        .await
    {
        Ok(job) => (StatusCode::OK, Json(export_job_response(job))).into_response(),
        Err(error) => access_error(error),
    }
}

pub(super) async fn authorize(
    state: &AppState,
    ctx: &AuthContext,
    job: &ExportJobRecord,
) -> Result<Option<InventoryExportArtifact>, Response> {
    if job.job_type != "INVENTORY_EXPORT" {
        return Ok(None);
    }
    ensure_permission(ctx, "inventory.read").map_err(IntoResponse::into_response)?;
    let spec = inventory_export_jobs::authorize_job(state.db.pool(), job, ctx.user_id, false)
        .await
        .map_err(access_error)?;
    if job.status == "DONE" && spec.artifact.is_none() {
        return Err(conflict("inventory export receipt missing").into_response());
    }
    Ok(spec.artifact)
}

pub(super) async fn visibility(
    state: &AppState,
    ctx: &AuthContext,
) -> Result<Option<InventoryExportVisibility>, Response> {
    if ensure_permission(ctx, "inventory.read").is_err() {
        return Ok(None);
    }
    inventory_export_jobs::visibility(state.db.pool(), ctx.tenant_id, ctx.user_id)
        .await
        .map_err(access_error)
}

pub(super) async fn check_path(path: &std::path::Path) -> std::io::Result<()> {
    for component in [
        Some(path),
        path.parent(),
        path.parent().and_then(std::path::Path::parent),
    ]
    .into_iter()
    .flatten()
    {
        let metadata = tokio::fs::symlink_metadata(component).await?;
        let linked = metadata.file_type().is_symlink();
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            linked || metadata.file_attributes() & 0x400 != 0
        };
        if linked {
            return Err(std::io::Error::other(
                "inventory export symlink/reparse path rejected",
            ));
        }
    }
    Ok(())
}
