use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn record_audit_log(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    action: &str,
    entity_type: &str,
    entity_id: Option<String>,
    before: Option<Value>,
    after: Option<Value>,
) {
    let repo = AuditLogsRepo::new(state.db.pool());
    let _ = repo
        .insert(AuditLogCreate {
            tenant_id: ctx.tenant_id,
            user_id: Some(ctx.user_id),
            action: action.to_string(),
            entity_type: entity_type.to_string(),
            entity_id,
            before_json: before,
            after_json: after,
            ip: req_ctx.ip.clone(),
            ua: req_ctx.user_agent.clone(),
        })
        .await;
}

pub(crate) fn token_audit_payload(token: &PlatformToken) -> Value {
    serde_json::json!({
        "has_access_token": token.access_token.is_some(),
        "has_refresh_token": token.refresh_token.is_some(),
        "expires_at": token.expires_at,
        "scopes": token.scopes.clone(),
        "token_status": token.token_status.clone(),
        "last_refreshed_at": token.last_refreshed_at,
        "refresh_fail_count": token.refresh_fail_count,
        "last_error": token.last_error.clone(),
        "updated_at": token.updated_at,
    })
}
