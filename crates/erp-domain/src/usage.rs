//! Resource metering. A quota is not a feature entitlement, and a replay is not a second charge.

/// Storage, export, MRP, attachment, and message are the metered kinds.
pub fn admit_resource_kind(kind: &str) -> Result<&'static str, &'static str> {
    match kind {
        "STORAGE" => Ok("STORAGE"),
        "EXPORT" => Ok("EXPORT"),
        "MRP" => Ok("MRP"),
        "ATTACHMENT" => Ok("ATTACHMENT"),
        "MESSAGE" => Ok("MESSAGE"),
        _ => Err("resource kind is not metered"),
    }
}

/// A meter row names the task, kind, quantity, and window. Quantity must be positive.
pub fn admit_usage_meter(task: &str, kind: &str, quantity: i64, window: &str) -> Result<&'static str, &'static str> {
    if task.trim().is_empty() || window.trim().is_empty() || quantity <= 0 {
        return Err("usage meter is incomplete");
    }
    admit_resource_kind(kind)
}

/// A repeated task returns the quantity already charged. It does not add a second row.
pub fn replayed_task_keeps_charge(stored: Option<i64>) -> Result<i64, &'static str> {
    match stored {
        Some(quantity) if quantity > 0 => Ok(quantity),
        _ => Err("a repeated task has no admitted charge"),
    }
}

/// A task past the cap is refused. The tasks already admitted stay.
pub fn refused_task_keeps_admitted(admitted_tasks: i64, cap: i64) -> Result<i64, &'static str> {
    if admitted_tasks < 0 || cap <= 0 {
        return Err("refused task cap is incomplete");
    }
    if admitted_tasks < cap {
        return Err("usage is still inside the task cap");
    }
    Ok(admitted_tasks)
}

/// A refused charge keeps the quantity already admitted. It does not drop that row.
pub fn refused_usage_keeps_admitted(admitted: i64, adding: i64, quota: i64) -> Result<i64, &'static str> {
    if admitted < 0 || adding <= 0 || quota <= 0 {
        return Err("refused usage is incomplete");
    }
    if admitted + adding <= quota {
        return Err("usage is still inside the quota");
    }
    Ok(admitted)
}

/// Over the quota is a notice, not a second charge and not a feature denial.
pub fn usage_decision(used: i64, adding: i64, quota: i64) -> Result<&'static str, &'static str> {
    if used < 0 || adding <= 0 || quota <= 0 {
        return Err("usage decision is incomplete");
    }
    if used + adding > quota { Ok("OVER_LIMIT") } else { Ok("ADMITTED") }
}

/// A stored notice is readable only when it is over the limit and offers a larger quota.
pub fn readable_usage_notice(decision: &str, expand: &str) -> Result<(&'static str, &'static str), &'static str> {
    if decision != "OVER_LIMIT" || expand != "EXPAND" {
        return Err("usage notice is not an over-limit expansion");
    }
    Ok(("OVER_LIMIT", "EXPAND"))
}

/// An expansion option names a larger quota. It does not grant a paid feature.
pub fn admit_expansion(current: i64, offered: i64) -> Result<&'static str, &'static str> {
    if current <= 0 || offered <= current {
        return Err("expansion must raise the resource quota");
    }
    Ok("EXPAND")
}

/// A resource quota and a feature entitlement stay separate. One cannot stand in for the other.
pub fn quota_is_entitlement(quota_code: &str, feature_code: &str) -> bool {
    !quota_code.trim().is_empty() && quota_code == feature_code
}
