//! Dedicated service terms. Receiving a ticket is not the same as resolving it.

/// A dedicated instance uses the same core version and a passed gate.
pub fn admit_dedicated_instance(core_version: &str, gate_passed: bool, storage_bytes: i64, backup_bytes: i64) -> Result<(), &'static str> {
    if core_version.trim().is_empty() || storage_bytes < 0 || backup_bytes < 0 {
        return Err("dedicated instance terms are incomplete");
    }
    if !gate_passed {
        return Err("a dedicated instance needs the same passed release gate");
    }
    Ok(())
}

/// A quote names the resource, term, deliverable, response, and exit path.
pub fn admit_service_quote(resource: &str, term_end: &str, deliverable: &str, response: &str, exit_path: &str) -> Result<(), &'static str> {
    if resource.trim().is_empty()
        || term_end.trim().is_empty()
        || deliverable.trim().is_empty()
        || response.trim().is_empty()
        || exit_path.trim().is_empty()
    {
        return Err("service quote is incomplete");
    }
    Ok(())
}

/// Opening a ticket records receipt only. Resolution is a later status.
pub fn support_ticket_decision(received: bool, resolved: bool) -> Result<&'static str, &'static str> {
    if !received {
        return Err("a support ticket must first be received");
    }
    if resolved { Ok("RESOLVED") } else { Ok("OPEN") }
}
