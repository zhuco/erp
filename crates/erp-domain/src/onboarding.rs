//! Enterprise opening. One verification opens one enterprise and can resume.

/// A verification token is single use. An empty identity or enterprise is refused.
/// An expiry at or before the open instant is already spent and cannot open an enterprise.
pub fn admit_onboarding_start(identity: &str, verification: &str, company_name: &str) -> Result<(), &'static str> {
    if identity.trim().is_empty() || verification.trim().is_empty() || company_name.trim().is_empty() {
        return Err("onboarding needs an identity, verification, and company name");
    }
    Ok(())
}

/// Opening is either trade, manufacturing, or both. It is not a paid seat change.
/// The credential is already expired when its expiry is at or before the open instant.
pub fn verification_still_open(expires_at: &str, opened_at: &str) -> Result<(), &'static str> {
    let expires_at = expires_at.trim();
    let opened_at = opened_at.trim();
    if expires_at.is_empty() || opened_at.is_empty() || expires_at <= opened_at {
        return Err("verification credential is expired");
    }
    Ok(())
}

/// Progress is the stored job and its steps. A page message is not a stage.
pub fn onboarding_progress<'a>(status: &'a str, steps: &'a [(&'a str, &'a str)]) -> (&'a str, &'a [(&'a str, &'a str)]) {
    (status, steps)
}

/// Opening is either trade, manufacturing, or both. It is not a paid seat change.
pub fn admit_company_mode(mode: &str, timezone: &str, currency: &str) -> Result<(), &'static str> {
    if !matches!(mode, "TRADE" | "MANUFACTURING" | "BOTH")
        || timezone.trim().is_empty()
        || currency.trim().is_empty()
    {
        return Err("company mode, timezone, or currency is incomplete");
    }
    Ok(())
}

/// A failed step stays resumable. Completion is a later explicit step.
pub fn onboarding_step_decision(step: &str, failed: bool) -> Result<&'static str, &'static str> {
    if !matches!(step, "VERIFY" | "COMPANY" | "INIT") {
        return Err("onboarding step is unknown");
    }
    if failed { Ok("FAILED") } else { Ok("OPEN") }
}
