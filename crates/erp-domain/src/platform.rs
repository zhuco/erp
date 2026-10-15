//! Platform interaction. Zoom changes the scale only. An unsupported feature records its real result.

/// 125 and 150 percent are recorded scales. Zero and a negative scale are refused.
pub fn admit_zoom(percent: i32) -> Result<i32, &'static str> {
    if !matches!(percent, 100 | 125 | 150) {
        return Err("zoom scale is not registered");
    }
    Ok(percent)
}

/// Zoom keeps the row id and the draft. It does not allocate a new identity.
pub fn zoom_keeps_identity(before_row: &str, after_row: &str, draft_before: &str, draft_after: &str) -> Result<(), &'static str> {
    if before_row.trim().is_empty() || before_row != after_row || draft_before != draft_after {
        return Err("zoom changed the row id or cleared the draft");
    }
    Ok(())
}

/// A semantic node names a real operable control. A decorative label is not a control.
pub fn admit_access_node(control_id: &str, role: &str, operable: bool) -> Result<&'static str, &'static str> {
    if control_id.trim().is_empty() || role.trim().is_empty() {
        return Err("access node is incomplete");
    }
    if !matches!(role, "GRID_CELL" | "COLUMN_HEADER" | "BUTTON" | "TEXT_INPUT") {
        return Err("access role is not registered");
    }
    if operable { Ok("OPERABLE") } else { Err("semantic node is not a real control") }
}

/// IME, clipboard, and screen reader each return the platform's real result.
pub fn platform_result(feature: &str, supported: bool) -> Result<&'static str, &'static str> {
    if !matches!(feature, "IME" | "CLIPBOARD" | "SCREEN_READER" | "FOCUS") {
        return Err("platform feature is not registered");
    }
    if supported { Ok("SUPPORTED") } else { Ok("UNSUPPORTED") }
}

/// A stored zoom is readable only when the row and draft are still the scaled page.
/// This is a stored read, not a drawn 125% or 150% window.
pub fn zoom_readable(percent: i32, row_id: &str, draft: &str) -> Result<(), &'static str> {
    admit_zoom(percent)?;
    if row_id.trim().is_empty() || draft.trim().is_empty() {
        return Err("zoom read lost the row or the draft");
    }
    Ok(())
}

/// A simulated event and a manual result stay separate. One cannot stand in for the other.
pub fn event_source(source: &str) -> Result<&'static str, &'static str> {
    match source {
        "SIMULATED" => Ok("SIMULATED"),
        "MANUAL" => Ok("MANUAL"),
        _ => Err("event source is not registered"),
    }
}
