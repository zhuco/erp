//! Service status and feedback. Public status does not carry customer detail.

/// Public status is a code only. Customer names and document bodies stay out.
pub fn public_status(code: &str) -> Result<&'static str, &'static str> {
    match code {
        "UP" => Ok("UP"),
        "DEGRADED" => Ok("DEGRADED"),
        "DOWN" => Ok("DOWN"),
        _ => Err("public status is unknown"),
    }
}

/// Feedback cites the failed request and the checklist item. It does not store the document body.
pub fn admit_feedback(request_ref: &str, feature_id: &str, document_no: &str) -> Result<(), &'static str> {
    if request_ref.trim().is_empty() || feature_id.trim().is_empty() || document_no.trim().is_empty() {
        return Err("feedback needs a request, a feature, and a document number");
    }
    if document_no.contains(' ') {
        return Err("feedback must not store the document body");
    }
    Ok(())
}

/// An internal ticket names scope, reason, handler, and result. Secrets are refused.
pub fn admit_ops_ticket(scope: &str, reason: &str, handler: &str, result: &str) -> Result<(), &'static str> {
    if scope.trim().is_empty() || reason.trim().is_empty() || handler.trim().is_empty() || result.trim().is_empty() {
        return Err("ops ticket is incomplete");
    }
    let lower = reason.to_ascii_lowercase();
    if lower.contains("password") || lower.contains("token") || lower.contains("secret") {
        return Err("ops ticket must not store a secret");
    }
    if !matches!(result, "OPEN" | "HANDLED") {
        return Err("ops result is unknown");
    }
    Ok(())
}

/// Diagnostics and notices stay inside a fixed bound.
pub fn diagnostic_within_bound(lines: i64, bound: i64) -> Result<(), &'static str> {
    if lines < 0 || bound < 1 || lines > bound {
        return Err("diagnostic export is over the bound");
    }
    Ok(())
}
