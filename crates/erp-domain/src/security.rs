//! Login backoff and rate limits. A load test cannot turn the checks off.

const LOGIN_BACKOFF_SECONDS: [i64; 5] = [1, 2, 4, 8, 16];

/// A failed login waits longer on each attempt. The secret itself is not stored.
pub fn login_backoff_seconds(failed_attempts: i32) -> Result<i64, &'static str> {
    if !(1..=5).contains(&failed_attempts) {
        return Err("login attempt is outside the backoff window");
    }
    Ok(LOGIN_BACKOFF_SECONDS[(failed_attempts - 1) as usize])
}

/// Stored credentials keep a reference, not the password or token value.
pub fn credential_reference(stored: &str) -> Result<&str, &'static str> {
    let stored = stored.trim();
    if stored.is_empty() || stored.contains("password") || stored.contains("token-value") {
        return Err("credential storage must not keep the secret");
    }
    Ok(stored)
}

/// A limit is counted per enterprise, account, and source. Over the limit is 429.
pub fn rate_limit_decision(tenant: &str, account: &str, source: &str, observed: i64, limit: i64) -> Result<&'static str, &'static str> {
    if tenant.trim().is_empty() || account.trim().is_empty() || source.trim().is_empty() || limit < 1 || observed < 0 {
        return Err("rate limit needs tenant, account, source, and a positive limit");
    }
    if observed >= limit { Ok("429") } else { Ok("ALLOWED") }
}

/// A new device is a notice. It does not become a second login seat.
pub fn unusual_device_notice(known: bool) -> &'static str {
    if known { "KNOWN" } else { "NOTICE" }
}

/// An official operator needs a factor. Revocation stays on the same row.
pub fn admin_factor_decision(factor_ok: bool, revoked: bool) -> Result<&'static str, &'static str> {
    if revoked {
        return Ok("REVOKED");
    }
    if factor_ok { Ok("VERIFIED") } else { Err("admin factor is missing") }
}

/// An admin token is outside the tenant business seats.
pub fn admin_token_is_business_seat() -> bool {
    false
}

/// Security checks stay on while measuring load.
pub fn security_check_stays_on(load_test: bool) -> Result<(), &'static str> {
    if load_test {
        return Err("security checks stay on during a load test");
    }
    Ok(())
}
