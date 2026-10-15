//! Page-instance workbench state. Hiding a panel keeps the draft. Closing a page does not cancel an accepted write.

/// A page instance names its page, instance, and a registered service. A blank service is not a page.
pub fn admit_page_instance(page: &str, instance: &str, service: &str) -> Result<(), &'static str> {
    if page.trim().is_empty() || instance.trim().is_empty() || service.trim().is_empty() {
        return Err("page instance is incomplete");
    }
    Ok(())
}

/// Light and dark are the only stored themes. Switching theme does not clear a draft.
pub fn admit_theme(theme: &str) -> Result<&'static str, &'static str> {
    match theme {
        "DARK" => Ok("DARK"),
        "LIGHT" => Ok("LIGHT"),
        _ => Err("theme is not registered"),
    }
}

/// A theme switch changes only the stored theme. The draft must still be the same text.
/// This is a stored switch, not a drawn theme change.
pub fn theme_switch_keeps_draft(before: &str, after: &str, from: &str, to: &str) -> Result<&'static str, &'static str> {
    let to = admit_theme(to)?;
    let _ = admit_theme(from)?;
    if before.trim().is_empty() || before != after {
        return Err("theme switch cleared the draft");
    }
    Ok(to)
}

/// A hidden panel keeps its query and draft. The visible flag is the only change.
pub fn panel_hide_keeps_state(query: &str, draft: &str) -> Result<(), &'static str> {
    if query.trim().is_empty() && draft.trim().is_empty() {
        return Err("hiding a panel cannot drop an empty stand-in for state");
    }
    Ok(())
}

/// A stored page is readable only when its query and draft are the caller's own.
pub fn page_state_belongs(stored_user: &str, caller: &str, query: &str, draft: &str) -> Result<(), &'static str> {
    if stored_user.trim().is_empty() || stored_user != caller || query.trim().is_empty() || draft.trim().is_empty() {
        return Err("page state is outside this user");
    }
    Ok(())
}

/// Closing a dirty page protects the draft. An accepted write is not cancelled by the close.
pub fn close_page_decision(dirty: bool, accepted_write: bool) -> Result<&'static str, &'static str> {
    if accepted_write {
        return Ok("WRITE_KEPT");
    }
    if dirty { Ok("DRAFT_KEPT") } else { Ok("CLOSED") }
}
