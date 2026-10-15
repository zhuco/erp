//! Data exit. A tenant export is a manifest, not another enterprise's database backup.

/// An accepted term names its version, purpose, and retention. It is not a blank page.
pub fn admit_service_term(version: &str, purpose: &str, retention: &str) -> Result<(), &'static str> {
    if version.trim().is_empty() || purpose.trim().is_empty() || retention.trim().is_empty() {
        return Err("service term is incomplete");
    }
    Ok(())
}

/// An exit request names the applicant, confirmation, cutoff, and file range.
pub fn admit_exit_request(applicant: &str, confirmation: &str, cutoff: &str, file_range: &str) -> Result<(), &'static str> {
    if applicant.trim().is_empty() || confirmation.trim().is_empty() || cutoff.trim().is_empty() || file_range.trim().is_empty() {
        return Err("exit request is incomplete");
    }
    if file_range == "DATABASE_BACKUP" {
        return Err("exit must not ship another enterprise database backup");
    }
    Ok(())
}

/// Exit progress is requested, then delivered. There is no wipe action.
pub fn exit_progress(delivered: bool) -> Result<&'static str, &'static str> {
    if delivered { Ok("DELIVERED") } else { Ok("REQUESTED") }
}

/// An unfinished exit must not present a file marker as if the data were taken.
pub fn exit_file_marker(status: &str, delivered: bool, marker: Option<String>) -> Option<String> {
    if status == "DONE" && delivered {
        marker.filter(|value| !value.trim().is_empty())
    } else {
        None
    }
}

/// A delivered file matches the manifest line count and digest. A mismatch is refused.
pub fn exit_file_matches(lines: i64, digest: &str, expected_lines: i64, expected_digest: &str) -> Result<(), &'static str> {
    if lines != expected_lines || digest != expected_digest || expected_digest.trim().is_empty() {
        return Err("delivered file does not match the exit manifest");
    }
    Ok(())
}

/// A manifest lists the current enterprise only. A foreign row is refused.
pub fn manifest_belongs(owner: &str, caller: &str) -> Result<(), &'static str> {
    if owner.trim().is_empty() || owner != caller {
        return Err("export manifest is outside this tenant");
    }
    Ok(())
}

/// Wiping the database is not an exit action.
pub fn database_wipe_allowed() -> bool {
    false
}
