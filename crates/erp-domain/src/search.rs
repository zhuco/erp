//! Saved search. An invalid range keeps the typed text and does not start a query.

/// Text, multi-select, date, and amount or quantity bounds are the stored condition kinds.
pub fn admit_condition_kind(kind: &str) -> Result<&'static str, &'static str> {
    match kind {
        "TEXT" => Ok("TEXT"),
        "MULTI" => Ok("MULTI"),
        "DATE" => Ok("DATE"),
        "AMOUNT" => Ok("AMOUNT"),
        "QTY" => Ok("QTY"),
        _ => Err("condition kind is not registered"),
    }
}

/// AND and OR are the only joins. A raw SQL fragment is refused.
pub fn admit_join(join: &str) -> Result<&'static str, &'static str> {
    match join {
        "AND" => Ok("AND"),
        "OR" => Ok("OR"),
        _ => Err("condition join is not registered"),
    }
}

/// An empty bound is open. A reversed or non-numeric amount stays typed and does not query.
/// Text, multi-select, and date keep their typed value and still query when it is not SQL.
pub fn range_decision(kind: &str, low: &str, high: &str) -> Result<&'static str, &'static str> {
    if low.contains(';') || high.contains(';') || low.contains("--") || high.contains("--") {
        return Err("condition text cannot carry SQL");
    }
    if !matches!(kind, "AMOUNT" | "QTY") {
        return Ok("QUERY");
    }
    let low_value = if low.trim().is_empty() { None } else { low.parse::<i64>().ok() };
    let high_value = if high.trim().is_empty() { None } else { high.parse::<i64>().ok() };
    if (!low.trim().is_empty() && low_value.is_none()) || (!high.trim().is_empty() && high_value.is_none()) {
        return Ok("KEPT");
    }
    if let (Some(low_value), Some(high_value)) = (low_value, high_value)
        && low_value > high_value
    {
        return Ok("KEPT");
    }
    Ok("QUERY")
}

/// A named view belongs to one enterprise, user, and page. A missing field is named, not injected.
pub fn admit_named_view(user: &str, page: &str, name: &str, field: &str, field_known: bool) -> Result<&'static str, &'static str> {
    if user.trim().is_empty() || page.trim().is_empty() || name.trim().is_empty() || field.trim().is_empty() {
        return Err("named view is incomplete");
    }
    if name.contains(';') || field.contains(';') {
        return Err("named view cannot carry SQL");
    }
    if field_known { Ok("SAVED") } else { Ok("FIELD_MISSING") }
}

/// Reset clears the stored conditions. It does not keep the previous invalid text.
pub fn reset_clears(previous: &str) -> Result<&'static str, &'static str> {
    if previous.trim().is_empty() {
        return Err("reset needs the previous condition");
    }
    Ok("CLEARED")
}

/// A kept invalid range still has the typed bounds. It is not an active query.
pub fn kept_range_readable(decision: &str, low: &str, high: &str) -> Result<(), &'static str> {
    if decision != "KEPT" || low.trim().is_empty() || high.trim().is_empty() {
        return Err("kept range is not readable");
    }
    if range_decision("AMOUNT", low, high)? != "KEPT" {
        return Err("kept range is not an invalid amount");
    }
    Ok(())
}

/// A late result may stay only when it is not applied and does not replace the current result.
pub fn late_result_not_applied(applied: bool, result: &str, current: &str) -> Result<(), &'static str> {
    if applied || result.trim().is_empty() || current.trim().is_empty() || result == current {
        return Err("late result would cover the current condition");
    }
    Ok(())
}

/// Reset is readable only after the previous bounds are gone and the decision is cleared.
pub fn reset_readable(decision: &str, low: &str, high: &str, previous: &str) -> Result<(), &'static str> {
    if decision != "CLEARED" || !low.is_empty() || !high.is_empty() || previous.trim().is_empty() {
        return Err("reset did not clear the previous condition");
    }
    Ok(())
}
