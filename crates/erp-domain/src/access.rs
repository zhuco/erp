//! Login-seat and session policy for the free manufacturing and trade product.
//!
//! A login seat is a business user who can authenticate. A session is one signed-in
//! client of that user. Device, address, and browser-tab counts are neither seats
//! nor sessions. Official operators are outside this tenant model.

use thiserror::Error;

/// Adopted free-plan policy: one business login, several clients at once.
/// Single-session remains a stored alternative and is not the free default.
pub const FREE_PLAN_CODE: &str = "FREE";
pub const PAID_PLAN_CODE: &str = "PAID";
pub const FREE_LOGIN_SEAT_LIMIT: i32 = 1;
/// Public resource ceiling. This is not an unlimited-session promise.
pub const FREE_MAX_ACTIVE_SESSIONS: i32 = 8;
pub const SESSION_TTL_SECONDS: i64 = 60 * 60;
pub const SESSION_POLICY_MULTI: &str = "MULTI";
pub const SESSION_POLICY_SINGLE: &str = "SINGLE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPolicy {
    Multi,
    Single,
}

impl SessionPolicy {
    pub fn parse(raw: &str) -> Result<Self, AccessError> {
        match raw.trim() {
            SESSION_POLICY_MULTI => Ok(Self::Multi),
            SESSION_POLICY_SINGLE => Ok(Self::Single),
            _ => Err(AccessError::InvalidPolicy),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Multi => SESSION_POLICY_MULTI,
            Self::Single => SESSION_POLICY_SINGLE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TenantAccessPlan {
    pub plan_code_free: bool,
    pub login_seat_limit: i32,
    pub session_policy: SessionPolicy,
    pub max_active_sessions: i32,
}

impl TenantAccessPlan {
    pub fn free_multi() -> Self {
        Self {
            plan_code_free: true,
            login_seat_limit: FREE_LOGIN_SEAT_LIMIT,
            session_policy: SessionPolicy::Multi,
            max_active_sessions: FREE_MAX_ACTIVE_SESSIONS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AccessError {
    #[error("login seat limit reached")]
    SeatLimitReached,
    #[error("active session limit reached")]
    SessionLimitReached,
    #[error("session is not active")]
    SessionInactive,
    #[error("session credentials are stale")]
    CredentialsStale,
    #[error("invalid session policy")]
    InvalidPolicy,
}

pub const PAID_SAMPLE_SEAT_LIMIT: i32 = 5;

/// An invitation does not occupy a seat until it is activated.
pub fn invitation_occupies_seat(status: &str) -> bool {
    status == "ACTIVE"
}

/// A paid plan may raise the seat limit. The free plan stays at one and cannot use paid approval.
pub fn admit_paid_plan(plan_code: &str, seat_limit: i32) -> Result<(), AccessError> {
    match plan_code {
        FREE_PLAN_CODE if seat_limit == FREE_LOGIN_SEAT_LIMIT => Ok(()),
        PAID_PLAN_CODE if (2..=32).contains(&seat_limit) => Ok(()),
        _ => Err(AccessError::InvalidPolicy),
    }
}

/// Activation counts only active seats. A pending invitation is not one of them.
pub fn admit_seat_activation(plan_code: &str, seat_limit: i32, occupied_active: i64) -> Result<(), AccessError> {
    admit_paid_plan(plan_code, seat_limit)?;
    if occupied_active < 0 || occupied_active >= i64::from(seat_limit) {
        return Err(AccessError::SeatLimitReached);
    }
    Ok(())
}

/// Approval names a submitter, an approver, a condition, and a version. The approver cannot be the submitter.
pub fn admit_approval(submitter: &str, approver: &str, condition: &str, version_no: i32) -> Result<(), AccessError> {
    if submitter.trim().is_empty() || approver.trim().is_empty() || condition.trim().is_empty() || version_no < 1 {
        return Err(AccessError::InvalidPolicy);
    }
    if submitter == approver {
        return Err(AccessError::InvalidPolicy);
    }
    Ok(())
}

/// Downgrade keeps the selected account and revokes the others. History is not deleted.
pub fn downgrade_account_decision(selected: bool) -> &'static str {
    if selected { "KEPT" } else { "REVOKED" }
}

/// `occupied_seats` counts users that can still log in. Disabled users do not occupy a seat.
pub fn admit_login_seat(plan: TenantAccessPlan, occupied_seats: i64) -> Result<(), AccessError> {
    if occupied_seats < 0 {
        return Err(AccessError::SeatLimitReached);
    }
    if occupied_seats >= i64::from(plan.login_seat_limit) {
        Err(AccessError::SeatLimitReached)
    } else {
        Ok(())
    }
}

/// A device session is not a login seat. Only an active business user occupies one.
pub fn device_session_occupies_seat(client_kind: &str) -> bool {
    let _ = client_kind;
    false
}

/// Disabling a seat keeps the row. The account can no longer log in.
pub fn disable_seat_decision(currently_active: bool) -> Result<&'static str, AccessError> {
    if currently_active { Ok("DISABLED") } else { Err(AccessError::SessionInactive) }
}

/// Opening a session never consumes a login seat.
pub fn admit_session(plan: TenantAccessPlan, active_sessions: i64) -> Result<(), AccessError> {
    if active_sessions < 0 {
        return Err(AccessError::SessionLimitReached);
    }
    if active_sessions >= i64::from(plan.max_active_sessions) {
        Err(AccessError::SessionLimitReached)
    } else {
        Ok(())
    }
}

/// Session renewal extends the same client. It is not a business command replay.
pub fn session_renewal_is_business_replay(command_kind: &str) -> Result<(), AccessError> {
    if command_kind == "RENEW" {
        Ok(())
    } else {
        Err(AccessError::CredentialsStale)
    }
}

/// Password recovery names the account and a one-time token. It does not open a second seat.
pub fn admit_password_recovery(user_id: &str, token: &str) -> Result<(), AccessError> {
    if user_id.trim().is_empty() || token.trim().is_empty() {
        return Err(AccessError::InvalidPolicy);
    }
    Ok(())
}

/// A presented token is usable only when the session row is still active and
/// was issued at the current credential epoch. Password change and disable bump
/// that epoch; signing out one session does not.
pub fn session_command_allowed(
    status_active: bool,
    not_expired: bool,
    token_epoch: i32,
    current_epoch: i32,
) -> Result<(), AccessError> {
    if !status_active || !not_expired {
        return Err(AccessError::SessionInactive);
    }
    if token_epoch != current_epoch {
        return Err(AccessError::CredentialsStale);
    }
    Ok(())
}

pub const PLATFORM_OPS: &str = "platform.ops";

/// A path tenant must match the authenticated tenant. A mismatch leaks nothing.
pub fn tenant_path_matches(authenticated: &str, path_tenant: &str) -> Result<(), &'static str> {
    if authenticated.trim().is_empty() || authenticated != path_tenant {
        return Err("authenticated tenant does not match the path");
    }
    Ok(())
}

/// A resource belongs to the caller only when both tenant keys match.
pub fn resource_belongs(owner_tenant: &str, caller_tenant: &str) -> Result<(), &'static str> {
    if owner_tenant.trim().is_empty() || owner_tenant != caller_tenant {
        return Err("resource is outside this tenant");
    }
    Ok(())
}

/// One action check. Tenant ownership is enforced by the caller before this rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionAccess {
    pub actor_user_id: String,
    pub permission: String,
    pub warehouse_id: String,
    pub granted: Vec<String>,
    pub warehouses: Vec<String>,
    pub platform_operator: bool,
}

/// A business action needs the actor, the permission, and a warehouse inside the scope.
/// Platform operations are refused for an ordinary account.
pub fn authorize_action(access: &ActionAccess) -> Result<(), &'static str> {
    if access.actor_user_id.trim().is_empty() || access.permission.trim().is_empty() {
        return Err("actor and permission are required");
    }
    if access.permission == PLATFORM_OPS {
        return if access.platform_operator {
            Ok(())
        } else {
            Err("platform operation is refused")
        };
    }
    if access.platform_operator {
        return Err("platform operator is outside tenant actions");
    }
    if !access.granted.iter().any(|item| item == &access.permission) {
        return Err("permission is missing");
    }
    if access.warehouse_id.trim().is_empty()
        || !access.warehouses.iter().any(|item| item == &access.warehouse_id)
    {
        return Err("warehouse is outside the data scope");
    }
    Ok(())
}

/// Audit keeps a short summary. Secrets and long customer detail are refused.
pub fn audit_summary(raw: &str) -> Result<String, &'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("audit summary is required");
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("password") || lower.contains("token") || lower.contains("secret") {
        return Err("audit must not store a secret");
    }
    if trimmed.chars().count() > 80 {
        return Err("audit summary is too detailed");
    }
    Ok(trimmed.to_string())
}

/// Free plan cannot turn audit off. Retention stays at least 30 days.
pub fn audit_retention_days(requested: i32, free_plan: bool) -> Result<i32, &'static str> {
    if requested <= 0 || (free_plan && requested < 30) {
        return Err("free plan cannot disable audit");
    }
    Ok(requested)
}

/// A frozen task scope is checked again at execution and download.
pub fn frozen_scope_still_allowed(frozen_warehouse: &str, current: &[String]) -> bool {
    !frozen_warehouse.is_empty() && current.iter().any(|item| item == frozen_warehouse)
}

/// Batch execution uses the same actor, permission, and warehouse check as a direct action.
pub fn batch_execution_allowed(access: &ActionAccess, frozen_warehouse: &str) -> Result<(), &'static str> {
    authorize_action(access)?;
    if access.warehouse_id != frozen_warehouse {
        return Err("frozen warehouse does not match the batch");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_plan_rejects_a_second_login_seat() {
        let plan = TenantAccessPlan::free_multi();
        assert!(admit_login_seat(plan, 0).is_ok());
        assert_eq!(
            admit_login_seat(plan, 1),
            Err(AccessError::SeatLimitReached)
        );
    }

    #[test]
    fn extra_sessions_do_not_consume_the_login_seat() {
        let plan = TenantAccessPlan::free_multi();
        assert!(admit_login_seat(plan, 1).is_err());
        assert!(admit_session(plan, 1).is_ok());
        assert!(admit_session(plan, 7).is_ok());
        assert_eq!(
            admit_session(plan, 8),
            Err(AccessError::SessionLimitReached)
        );
    }

    #[test]
    fn one_logout_does_not_invalidate_other_sessions() {
        assert!(session_command_allowed(true, true, 3, 3).is_ok());
        assert_eq!(
            session_command_allowed(false, true, 3, 3),
            Err(AccessError::SessionInactive)
        );
    }

    #[test]
    fn password_change_rejects_tokens_from_the_previous_epoch() {
        assert_eq!(
            session_command_allowed(true, true, 3, 4),
            Err(AccessError::CredentialsStale)
        );
    }
}
