use super::*;
use erp_repo::inventory_movement_commands::{
    InventoryCommandError, InventoryCommandResult, InventoryMovementCommandInput,
    InventoryMovementCommandsRepo, normalize_inventory_request_id,
};

pub(crate) async fn inventory_command_warehouse_access(
    state: &AppState,
    ctx: &AuthContext,
    tenant: TenantId,
    warehouse: uuid::Uuid,
) -> Result<(), Response> {
    if let Err(response) = ensure_tenant_access(ctx, tenant) {
        return Err(response.into_response());
    }
    if let Err(response) = ensure_warehouse_access(ctx, warehouse) {
        return Err(response.into_response());
    }
    if !ctx.is_admin {
        let granted: bool = sqlx::query_scalar(
            "select exists(select 1 from user_warehouse_access g
             join warehouses w on w.tenant_id=g.tenant_id and w.warehouse_id=g.warehouse_id
             where g.tenant_id=$1 and g.user_id=$2 and g.warehouse_id=$3)",
        )
        .bind(tenant.0)
        .bind(ctx.user_id.0)
        .bind(warehouse)
        .fetch_one(state.db.pool())
        .await
        .map_err(|error| (StatusCode::SERVICE_UNAVAILABLE, format!("{error}")).into_response())?;
        if !granted {
            return Err(forbidden("warehouse access denied").into_response());
        }
    }
    Ok(())
}

pub(crate) async fn execute_inventory_command(
    state: &AppState,
    ctx: &AuthContext,
    input: InventoryMovementCommandInput,
) -> Response {
    if let Err(response) = ensure_permission(ctx, "inventory.write") {
        return response.into_response();
    }
    if let Err(response) =
        inventory_command_warehouse_access(state, ctx, input.tenant_id, input.warehouse_id).await
    {
        return response;
    }
    let enabled =
        match feature_flag_enabled(state, input.tenant_id, None, FeatureFlagKey::InventoryWrite)
            .await
        {
            Ok(value) => value,
            Err(response) => return response.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }
    match InventoryMovementCommandsRepo::new(state.db.pool())
        .execute(input)
        .await
    {
        Ok(result) => inventory_command_response(result),
        Err(error) => inventory_command_error(error),
    }
}

pub(crate) async fn get_inventory_command(
    State(state): State<AppState>,
    Path((tenant, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> Response {
    let tenant = match TenantId::from_str(&tenant) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "inventory.read") {
        return response.into_response();
    }
    let request_id = match normalize_inventory_request_id(&request_id) {
        Ok(value) => value,
        Err(error) => return inventory_command_error(error),
    };
    let result = match InventoryMovementCommandsRepo::new(state.db.pool())
        .find(tenant, ctx.user_id, &request_id)
        .await
    {
        Ok(Some(result)) => result,
        Ok(None) => return not_found("inventory command not found").into_response(),
        Err(error) => return inventory_command_error(error),
    };
    if let Err(response) =
        inventory_command_warehouse_access(&state, &ctx, tenant, result.snapshot.input.warehouse_id)
            .await
    {
        return response;
    }
    inventory_command_response(result)
}

fn inventory_command_response(result: InventoryCommandResult) -> Response {
    let snapshot = result.snapshot;
    let mut response = serde_json::json!({
        "request_id":snapshot.input.request_id,"status":"SUCCEEDED","replay":result.replay,
        "effect_id":snapshot.ledger_id,"payload_hash":result.payload_hash,
        "operation_log_id":snapshot.operation_log_id,"audit_id":snapshot.audit_id,
        "record":inventory_balance_response(snapshot.balance),
    });
    if !snapshot.batch_effects.is_empty() {
        response["batch_id"] = serde_json::json!(snapshot.input.batch_id);
        response["batch_effects"] = serde_json::json!(snapshot.batch_effects);
    }
    if let Some(effect) = snapshot.location_effect {
        response["location"] = serde_json::json!(snapshot.input.location);
        response["location_record"] = serde_json::json!(effect);
        response["ledger_id"] = serde_json::json!(effect.ledger_id);
        response["qty_micros"] =
            serde_json::json!(snapshot.input.quantity_delta.checked_mul(1_000_000));
        response["bucket"] = serde_json::json!("ON_HAND");
    }
    if let Some(effect) = snapshot.reservation_effect {
        response["location"] = serde_json::json!(snapshot.input.location);
        response["reservation_record"] = serde_json::json!(effect);
        response["ledger_id"] = serde_json::json!(effect.ledger_id);
        response["source_line"] = serde_json::json!(effect.source_line);
        response["qty_micros"] =
            serde_json::json!(snapshot.input.quantity_delta.checked_mul(1_000_000));
        response["bucket"] = serde_json::json!("RESERVED");
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) fn inventory_command_error(error: anyhow::Error) -> Response {
    match error.downcast_ref::<InventoryCommandError>() {
        Some(InventoryCommandError::InvalidPayload(message)) => {
            bad_request(message).into_response()
        }
        Some(InventoryCommandError::IdentityConflict) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"code":"inventory_command_identity_conflict"})),
        )
            .into_response(),
        Some(InventoryCommandError::ReconciliationRequired) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"code":"inventory_command_reconciliation_required"})),
        )
            .into_response(),
        Some(InventoryCommandError::InsufficientLocationStock {
            available_micros,
            requested_micros,
        }) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "code":"inventory_location_insufficient_available",
                "available_micros":available_micros.to_string(),
                "requested_micros":requested_micros.to_string(),
            })),
        )
            .into_response(),
        Some(InventoryCommandError::QualityNotIssuable) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"code":"inventory_location_quality_not_issuable"})),
        )
            .into_response(),
        Some(InventoryCommandError::ReservationSourceConflict) => (
            StatusCode::CONFLICT,Json(serde_json::json!({"code":"inventory_reservation_source_conflict"})),
        ).into_response(),
        Some(InventoryCommandError::ReservationRemainingInsufficient {remaining_micros,requested_micros}) => (
            StatusCode::CONFLICT,Json(serde_json::json!({"code":"inventory_reservation_insufficient_remaining",
                "remaining_micros":remaining_micros.to_string(),"requested_micros":requested_micros.to_string()})),
        ).into_response(),
        Some(InventoryCommandError::RolePermissionDenied) => {
            forbidden("current role does not hold inventory write").into_response()
        }
        Some(InventoryCommandError::SessionInactive) => {
            (StatusCode::UNAUTHORIZED, "session is not active").into_response()
        }
        Some(InventoryCommandError::CredentialsStale) => {
            (StatusCode::UNAUTHORIZED, "credentials changed").into_response()
        }
        None => inventory_error_response(error).into_response(),
    }
}
