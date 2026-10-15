//! Company settings. A later version does not rewrite posted documents.

/// A setting names the company, numbering, currency, timezone, and precision.
pub fn admit_company_setting(
    name: &str,
    numbering: &str,
    currency: &str,
    timezone: &str,
    precision: i32,
    effective_at: &str,
) -> Result<(), &'static str> {
    if name.trim().is_empty()
        || numbering.trim().is_empty()
        || currency.trim().is_empty()
        || timezone.trim().is_empty()
        || effective_at.trim().is_empty()
        || !(0..=6).contains(&precision)
    {
        return Err("company setting is incomplete");
    }
    Ok(())
}

/// Trade, make, and resale can be combined. At least one mode stays on.
pub fn admit_company_modes(buy: bool, make: bool, sell: bool) -> Result<(), &'static str> {
    if !buy && !make && !sell {
        return Err("a company needs at least one business mode");
    }
    Ok(())
}

/// A demo action stays in the demo scope. It cannot post the real ledger.
pub fn demo_action_posts_real_ledger(scope: &str) -> Result<bool, &'static str> {
    match scope {
        "DEMO" => Ok(false),
        "REAL" => Ok(true),
        _ => Err("scope must be DEMO or REAL"),
    }
}

/// Leaving the demo keeps real imports and the first real document.
pub fn demo_exit_keeps_real(scope: &str) -> Result<(), &'static str> {
    if scope != "REAL" {
        return Err("only a real row survives leaving the demo");
    }
    Ok(())
}

/// A trade document does not need a BOM. Manufacturing history stays queryable after make is off.
pub fn trade_document_needs_bom(make_enabled: bool) -> bool {
    let _ = make_enabled;
    false
}
