use super::*;
use sha2::Digest;

#[derive(Debug, Deserialize)]
pub(crate) struct LoginRequest {
    tenant_id: String,
    email: String,
    password: String,
    client_kind: Option<String>,
    client_label: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct LoginResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: u64,
    session_id: String,
    session_policy: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurrentUserUpdateRequest {
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurrentUserPasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SessionRenewRequest {
    session_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PasswordRecoverRequest {
    tenant_id: String,
    email: String,
    token: String,
    new_password: String,
}

pub(crate) async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&req.tenant_id) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad tenant_id").into_response(),
    };

    let auth_repo = AuthRepo::new(state.db.pool());
    let user = match auth_repo.find_user_by_email(tenant_id, &req.email).await {
        Ok(Some(v)) => v,
        Ok(None) => return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if user.status != "ACTIVE" {
        return (StatusCode::UNAUTHORIZED, "user disabled").into_response();
    }
    let source = req.client_kind.as_deref().unwrap_or("UNKNOWN").trim();
    if let Some(response) =
        login_backoff_still_open(&state, tenant_id, req.email.trim(), source).await
    {
        return response;
    }

    match verify_password_off_executor(req.password.clone(), user.password_hash.clone()).await {
        Ok(true) => {}
        Ok(false) => return record_formal_login_failure(&state, tenant_id, &req).await,
        Err(PasswordVerificationError::Busy) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "code":"login_busy","message":"login temporarily unavailable"
                })),
            )
                .into_response();
        }
        Err(PasswordVerificationError::Task) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "password verification unavailable",
            )
                .into_response();
        }
    }

    let perms = match auth_repo.list_permissions(tenant_id, user.user_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let client_kind = req.client_kind.as_deref().unwrap_or("UNKNOWN");
    let client_label = req.client_label.as_deref().unwrap_or("");
    let expires_at =
        Utc::now() + chrono::Duration::seconds(erp_domain::access::SESSION_TTL_SECONDS);
    let session = match AccessPlanRepo::new(state.db.pool())
        .open_session(
            tenant_id,
            user.user_id,
            &user.password_hash,
            client_kind,
            client_label,
            expires_at,
        )
        .await
    {
        Ok(Ok(session)) => session,
        Ok(Err(erp_domain::access::AccessError::SessionLimitReached)) => {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "code": "session_limit",
                    "message": "active session limit reached"
                })),
            )
                .into_response();
        }
        Ok(Err(_)) => return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let now = Utc::now().timestamp() as usize;
    let exp = now + erp_domain::access::SESSION_TTL_SECONDS as usize;
    let claims = Claims {
        sub: user.user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        perms,
        is_admin: false,
        iat: now,
        exp,
        sid: Some(session.session_id.to_string()),
        cep: Some(session.credentials_epoch),
    };

    let token = match state.jwt.encode(&claims) {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    Json(LoginResponse {
        access_token: token,
        token_type: "Bearer",
        expires_in: erp_domain::access::SESSION_TTL_SECONDS as u64,
        session_id: session.session_id.to_string(),
        session_policy: erp_domain::access::SESSION_POLICY_MULTI.to_string(),
    })
    .into_response()
}

enum PasswordVerificationError {
    Busy,
    Task,
}

async fn verify_password_off_executor(
    password: String,
    hash: String,
) -> Result<bool, PasswordVerificationError> {
    static BUDGET: std::sync::OnceLock<PasswordVerificationBudget> = std::sync::OnceLock::new();
    BUDGET
        .get_or_init(PasswordVerificationBudget::new)
        .verify(password, hash)
        .await
}

#[derive(Clone)]
struct PasswordVerificationBudget {
    queue: Arc<tokio::sync::Semaphore>,
    running: Arc<tokio::sync::Semaphore>,
}

impl PasswordVerificationBudget {
    fn new() -> Self {
        Self {
            queue: Arc::new(tokio::sync::Semaphore::new(32)),
            running: Arc::new(tokio::sync::Semaphore::new(2)),
        }
    }

    async fn verify(
        &self,
        password: String,
        hash: String,
    ) -> Result<bool, PasswordVerificationError> {
        // Bound both waiting work and running Argon2 memory. Permits move into the
        // blocking job: cancelling its caller must not admit another hash before
        // the already-running hash actually finishes.
        let admitted = Arc::clone(&self.queue)
            .try_acquire_owned()
            .map_err(|_| PasswordVerificationError::Busy)?;
        let running = Arc::clone(&self.running)
            .acquire_owned()
            .await
            .map_err(|_| PasswordVerificationError::Task)?;
        tokio::task::spawn_blocking(move || {
            let _admitted = admitted;
            let _running = running;
            match argon2::PasswordHash::new(&hash) {
                Ok(parsed) => verify_login_password(password.as_bytes(), &parsed).is_ok(),
                Err(_) => false,
            }
        })
        .await
        .map_err(|_| PasswordVerificationError::Task)
    }
}

#[cfg(test)]
mod password_verification_tests {
    use super::{PasswordVerificationBudget, PasswordVerificationError};
    use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
    use std::{sync::Arc, time::Duration};

    #[tokio::test]
    async fn auth_password_budget_queue_rejection_and_cancel_return_capacity() -> anyhow::Result<()>
    {
        const PASSWORD: &str = "isolated-API-password-budget-correct-2026!";
        let hash = tokio::task::spawn_blocking(|| {
            let salt = SaltString::generate(&mut rand::thread_rng());
            Argon2::default()
                .hash_password(PASSWORD.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|error| anyhow::anyhow!("owned QA hash failed: {error}"))
        })
        .await??;
        let budget = PasswordVerificationBudget::new();
        // Own both running slots so each admitted real verification must wait.
        let running = Arc::clone(&budget.running).acquire_many_owned(2).await?;
        let mut pending = Vec::new();
        for _ in 0..32 {
            let budget = budget.clone();
            let hash = hash.clone();
            pending.push(tokio::spawn(async move {
                budget.verify(PASSWORD.into(), hash).await
            }));
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while budget.queue.available_permits() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        assert!(
            matches!(
                budget.verify(PASSWORD.into(), hash.clone()).await,
                Err(PasswordVerificationError::Busy)
            ),
            "request 33 must fail before adding another waiting verification"
        );
        let survivor = pending.remove(0);
        for task in pending {
            task.abort();
            assert!(task.await.err().is_some_and(|error| error.is_cancelled()));
        }
        assert_eq!(
            budget.queue.available_permits(),
            31,
            "cancelled waiting work returns its admission slots"
        );
        assert_eq!(
            budget.running.available_permits(),
            0,
            "waiting cancellation cannot release someone else's running slots"
        );
        drop(running);
        assert!(
            matches!(survivor.await?, Ok(true)),
            "the surviving queued request must actually verify the real salted password"
        );
        assert!(
            matches!(
                budget.verify("owned-QA-wrong-password".into(), hash).await,
                Ok(false)
            ),
            "a completed worker must retain the invalid-password result"
        );
        assert_eq!(budget.queue.available_permits(), 32);
        assert_eq!(budget.running.available_permits(), 2);
        Ok(())
    }
}

async fn login_backoff_still_open(
    state: &AppState,
    tenant_id: TenantId,
    account: &str,
    source: &str,
) -> Option<Response> {
    if account.is_empty() || source.is_empty() {
        return None;
    }
    let row: Option<(i64, chrono::DateTime<Utc>)> = sqlx::query_as(
        "select backoff_seconds, created_at from login_attempts
         where tenant_id = $1 and account_id = $2 and source_text = $3
         order by failed_count desc limit 1",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(account)
    .bind(source)
    .fetch_optional(state.db.pool())
    .await
    .ok()
    .flatten();
    let (wait, created_at) = row?;
    let elapsed = Utc::now().signed_duration_since(created_at).num_seconds();
    let remaining = wait.saturating_sub(elapsed);
    if remaining <= 0 {
        return None;
    }
    Some(
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "code": "login_backoff",
                "retry_after_seconds": remaining
            })),
        )
            .into_response(),
    )
}

async fn record_formal_login_failure(
    state: &AppState,
    tenant_id: TenantId,
    req: &LoginRequest,
) -> Response {
    let account = req.email.trim();
    let source = req.client_kind.as_deref().unwrap_or("UNKNOWN").trim();
    let device = req.client_label.as_deref().unwrap_or("").trim();
    if account.is_empty() || source.is_empty() || device.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"code":"invalid_credentials"})),
        )
            .into_response();
    }
    let prior: i64 = match sqlx::query_scalar(
        "select count(*) from login_attempts where tenant_id = $1 and account_id = $2 and source_text = $3",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(account)
    .bind(source)
    .fetch_one(state.db.pool())
    .await
    {
        Ok(count) => count,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"code":"invalid_credentials"})),
            )
                .into_response();
        }
    };
    let failed_count = i32::try_from(prior.saturating_add(1)).unwrap_or(6);
    let request_id = format!("login-fail:{account}:{source}:{failed_count}");
    let secret_ref = format!(
        "login-ref:{}",
        hex_encode(&sha2::Sha256::digest(account.as_bytes()))
    );
    match SharedContractRepo::new(state.db.pool())
        .record_login_failure(
            tenant_id,
            &request_id,
            &request_id,
            account,
            source,
            device,
            &secret_ref,
            failed_count,
        )
        .await
    {
        Ok(Ok((_, wait))) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "code": "invalid_credentials",
                "retry_after_seconds": wait
            })),
        )
            .into_response(),
        Ok(Err(_)) | Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"code":"invalid_credentials"})),
        )
            .into_response(),
    }
}

fn verify_login_password(
    password: &[u8],
    parsed: &argon2::PasswordHash<'_>,
) -> Result<(), argon2::password_hash::Error> {
    // Isolated QA may set a lower memory cost. Production hashes keep the default.
    if std::env::var("ERP_GRID_TEST_DATABASE_URL").is_ok() {
        if let Some(memory) = parsed
            .params
            .get("m")
            .and_then(|value| value.decimal().ok())
            .filter(|kib| *kib > 0 && *kib <= 8192)
        {
            let params = argon2::Params::new(memory, 1, 1, None)?;
            let cheap =
                argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
            return cheap.verify_password(password, parsed);
        }
    }
    argon2::Argon2::default().verify_password(password, parsed)
}

pub(crate) async fn list_own_sessions(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match AccessPlanRepo::new(state.db.pool())
        .list_own_sessions(ctx.tenant_id, ctx.user_id)
        .await
    {
        Ok(sessions) => {
            let body: Vec<_> = sessions
                .into_iter()
                .map(|session| {
                    serde_json::json!({
                        "session_id": session.session_id,
                        "status": session.status,
                        "client_kind": session.client_kind,
                        "client_label": session.client_label,
                        "created_at": session.created_at,
                        "expires_at": session.expires_at,
                    })
                })
                .collect();
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

pub(crate) async fn revoke_own_session(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let session_id = match uuid::Uuid::parse_str(&session_id) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad session_id").into_response(),
    };
    match AccessPlanRepo::new(state.db.pool())
        .revoke_own_session(ctx.tenant_id, ctx.user_id, session_id)
        .await
    {
        Ok(true) => (StatusCode::OK, Json(serde_json::json!({"revoked": true}))).into_response(),
        Ok(false) => not_found("session not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

pub(crate) async fn get_current_user(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let repo = RbacRepo::new(state.db.pool());
    let user = match repo.get_user(ctx.tenant_id, ctx.user_id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::UNAUTHORIZED, "unauthorized", "user missing")
                .into_response();
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let roles = match repo.list_user_roles(ctx.tenant_id, ctx.user_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = CurrentUserResponse {
        user,
        roles,
        permissions: ctx.perms.clone(),
        data_scopes: DataScopeResponse {
            shop_ids: ctx.shop_ids.iter().map(|id| id.to_string()).collect(),
            warehouse_ids: ctx.warehouse_ids.iter().map(|id| id.to_string()).collect(),
            merchant_ids: ctx.merchant_ids.iter().map(|id| id.to_string()).collect(),
            organization_ids: ctx
                .organization_ids
                .iter()
                .map(|id| id.to_string())
                .collect(),
        },
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn update_current_user_profile(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<CurrentUserUpdateRequest>,
) -> impl IntoResponse {
    let email = match req.email.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => return bad_request("email required").into_response(),
    };

    let repo = RbacRepo::new(state.db.pool());
    let before = match repo.get_user(ctx.tenant_id, ctx.user_id).await {
        Ok(Some(user)) => serde_json::to_value(user).ok(),
        Ok(None) => return not_found("user not found").into_response(),
        Err(_) => None,
    };

    let user = match repo
        .update_user(
            ctx.tenant_id,
            ctx.user_id,
            UserUpdate {
                email: Some(email),
                password_hash: None,
                status: None,
            },
        )
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => return not_found("user not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.USER.PROFILE.UPDATE",
        "user",
        Some(user.user_id.to_string()),
        before,
        Some(serde_json::to_value(&user).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(user)).into_response()
}

pub(crate) async fn update_current_user_password(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<CurrentUserPasswordRequest>,
) -> impl IntoResponse {
    let current_password = req.current_password.trim();
    let new_password = req.new_password.trim();
    if current_password.is_empty() {
        return bad_request("current_password required").into_response();
    }
    if new_password.is_empty() {
        return bad_request("new_password required").into_response();
    }

    let auth_repo = AuthRepo::new(state.db.pool());
    let user_row = match auth_repo.get_user_by_id(ctx.tenant_id, ctx.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return not_found("user not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let parsed_hash = match argon2::PasswordHash::new(&user_row.password_hash) {
        Ok(v) => v,
        Err(_) => return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
    };
    if verify_login_password(current_password.as_bytes(), &parsed_hash).is_err() {
        return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    }

    let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());
    let hash = match argon2::Argon2::default().hash_password(new_password.as_bytes(), &salt) {
        Ok(v) => v.to_string(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "hash failed").into_response(),
    };

    let repo = RbacRepo::new(state.db.pool());
    let before = match repo.get_user(ctx.tenant_id, ctx.user_id).await {
        Ok(Some(user)) => serde_json::to_value(user).ok(),
        Ok(None) => return not_found("user not found").into_response(),
        Err(_) => None,
    };

    // Hash replacement and epoch advance commit together. A failed epoch bump
    // rolls back the new hash, so an old session cannot outlive a stored password.
    let mut tx = match state.db.pool().begin().await {
        Ok(tx) => tx,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let user = match repo
        .update_user_in_tx(
            &mut tx,
            ctx.tenant_id,
            ctx.user_id,
            UserUpdate {
                email: None,
                password_hash: Some(hash),
                status: None,
            },
        )
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => return not_found("user not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let epoch = match AccessPlanRepo::new(state.db.pool())
        .bump_credentials_epoch_in_tx(&mut tx, ctx.tenant_id)
        .await
    {
        Ok(epoch) => epoch,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    if let Err(error) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.USER.PASSWORD.UPDATE",
        "user",
        Some(user.user_id.to_string()),
        before,
        Some(serde_json::to_value(&user).unwrap_or(Value::Null)),
    )
    .await;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "user_id": user.user_id,
            "email": user.email,
            "status": user.status,
            "credentials_epoch": epoch,
        })),
    )
        .into_response()
}

/// Extends the caller's own formal session and returns a usable token.
/// The old token is not a business command, so renewal does not replay one.
pub(crate) async fn renew_own_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SessionRenewRequest>,
) -> impl IntoResponse {
    let bearer = match headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
        }) {
        Some(token) if !token.is_empty() => token.to_string(),
        _ => return (StatusCode::UNAUTHORIZED, "missing Authorization").into_response(),
    };
    // Expiry is what renewal repairs. Signature and session identity still bind the caller.
    let claims = match state.jwt.decode_ignoring_exp(&bearer) {
        Ok(claims) => claims,
        Err(_) => return (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
    };
    if claims.is_admin {
        return (StatusCode::UNAUTHORIZED, "session required").into_response();
    }
    let tenant_id = match TenantId::from_str(&claims.tenant_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "bad tenant_id in token").into_response(),
    };
    let user_id = match UserId::from_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "bad sub in token").into_response(),
    };
    let token_session = match claims
        .sid
        .as_deref()
        .and_then(|sid| uuid::Uuid::parse_str(sid).ok())
    {
        Some(id) if !id.is_nil() => id,
        _ => return (StatusCode::UNAUTHORIZED, "bad session").into_response(),
    };
    let requested = match uuid::Uuid::parse_str(req.session_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad session_id").into_response(),
    };
    if requested != token_session {
        return forbidden("session mismatch").into_response();
    }
    let token_epoch = match claims.cep {
        Some(epoch) => epoch,
        None => return (StatusCode::UNAUTHORIZED, "credentials changed").into_response(),
    };
    let current_epoch = match AccessPlanRepo::new(state.db.pool())
        .load_plan(tenant_id)
        .await
    {
        Ok(Some(plan)) => plan.credentials_epoch,
        Ok(None) => return (StatusCode::UNAUTHORIZED, "session revoked").into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    if erp_domain::access::session_command_allowed(true, true, token_epoch, current_epoch).is_err()
    {
        return (StatusCode::UNAUTHORIZED, "credentials changed").into_response();
    }
    let expires_at =
        Utc::now() + chrono::Duration::seconds(erp_domain::access::SESSION_TTL_SECONDS);
    let session = match AccessPlanRepo::new(state.db.pool())
        .renew_session(tenant_id, user_id, token_session, expires_at)
        .await
    {
        Ok(Ok(session)) => session,
        Ok(Err(_)) => return (StatusCode::UNAUTHORIZED, "session revoked").into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let perms = match AuthRepo::new(state.db.pool())
        .list_permissions(tenant_id, user_id)
        .await
    {
        Ok(perms) => perms,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        perms,
        is_admin: false,
        iat: now,
        exp: now + erp_domain::access::SESSION_TTL_SECONDS as usize,
        sid: Some(session.session_id.to_string()),
        cep: Some(session.credentials_epoch),
    };
    let token = match state.jwt.encode(&claims) {
        Ok(token) => token,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    Json(LoginResponse {
        access_token: token,
        token_type: "Bearer",
        expires_in: erp_domain::access::SESSION_TTL_SECONDS as u64,
        session_id: session.session_id.to_string(),
        session_policy: erp_domain::access::SESSION_POLICY_MULTI.to_string(),
    })
    .into_response()
}

/// Consumes one recovery token and replaces the formal password.
/// The response never echoes the token or the new password.
pub(crate) async fn recover_own_password(
    State(state): State<AppState>,
    Json(req): Json<PasswordRecoverRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(req.tenant_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let email = req.email.trim();
    let token = req.token.trim();
    let new_password = req.new_password.trim();
    if email.is_empty() || token.is_empty() || new_password.is_empty() {
        return bad_request("email, token and new_password required").into_response();
    }
    if erp_domain::access::admit_password_recovery(email, token).is_err() {
        return bad_request("password recovery is incomplete").into_response();
    }
    let user = match AuthRepo::new(state.db.pool())
        .find_user_by_email(tenant_id, email)
        .await
    {
        Ok(Some(user)) if user.status == "ACTIVE" => user,
        Ok(_) => return (StatusCode::UNAUTHORIZED, "invalid recovery").into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());
    let hash = match argon2::Argon2::default().hash_password(new_password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "hash failed").into_response(),
    };
    let token_hash = sha2::Sha256::digest(format!("{tenant_id}:{email}:{token}").as_bytes());
    let token_hash = hex_encode(&token_hash);
    match AccessPlanRepo::new(state.db.pool())
        .recover_formal_password(tenant_id, user.user_id, &token_hash, &hash)
        .await
    {
        Ok(Ok(epoch)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "user_id": user.user_id,
                "status": "RECOVERED",
                "credentials_epoch": epoch,
            })),
        )
            .into_response(),
        Ok(Err(_)) => (StatusCode::UNAUTHORIZED, "invalid recovery").into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response(),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
