//! Unsubmitted drafts. An accepted server write is not cancelled by local undo or closing the page.

/// A draft names the enterprise, user, page, and object version. A blank scope is not recoverable.
pub fn admit_draft_scope(user: &str, page: &str, object_id: &str, version: i64) -> Result<(), &'static str> {
    if user.trim().is_empty() || page.trim().is_empty() || object_id.trim().is_empty() || version < 1 {
        return Err("draft scope is incomplete");
    }
    Ok(())
}

/// Restore rechecks the current object version. A stale draft is not applied.
pub fn restore_decision(draft_version: i64, current_version: i64) -> Result<&'static str, &'static str> {
    if draft_version < 1 || current_version < 1 {
        return Err("restore version is missing");
    }
    if draft_version == current_version { Ok("RECHECKED") } else { Ok("STALE") }
}

/// Closing the window looks up an unknown command before any new write.
/// A clean page may close. An unknown result must be looked up. A stored result is kept.
pub fn window_close_decision(command_known: bool, client_unknown: bool) -> &'static str {
    if client_unknown {
        "LOOKUP"
    } else if command_known {
        "KEEP"
    } else {
        "CLOSE"
    }
}

/// Local undo and closing the page can drop only an unsubmitted draft.
pub fn undo_decision(accepted_write: bool, action: &str) -> Result<&'static str, &'static str> {
    if !matches!(action, "UNDO" | "CLOSE") {
        return Err("draft action is not registered");
    }
    if accepted_write { Err("an accepted write is not cancelled by local undo") } else { Ok("DRAFT_DROPPED") }
}

/// A lost response is looked up once. The lookup does not post a second effect.
pub fn lost_response_lookup(stored_status: &str) -> Result<&'static str, &'static str> {
    match stored_status {
        "ACCEPTED" | "SUCCEEDED" => Ok("LOOKED_UP"),
        _ => Err("lost response needs a stored command"),
    }
}

/// A readable lookup names one stored effect. A second effect is not a lookup.
pub fn lost_lookup_readable(status: &str, effect: &str, effect_count: i64) -> Result<(), &'static str> {
    if !matches!(status, "ACCEPTED" | "SUCCEEDED") || effect.trim().is_empty() || effect_count != 1 {
        return Err("lost lookup is not one stored effect");
    }
    Ok(())
}

/// A draft is readable only inside its own enterprise. Another enterprise is not this draft.
pub fn draft_space_readable(body: &str, owner: &str, requested: &str) -> Result<(), &'static str> {
    if body.trim().is_empty() || owner.trim().is_empty() || owner != requested {
        return Err("draft belongs to another enterprise");
    }
    Ok(())
}

/// A refused quantity stays as the held draft. A stored conversion or sample is not a kept draft.
/// `written` is the conversion or quantity-sample count for this request. It must stay zero.
pub fn refused_qty_keeps_draft(refused: &str, held: &str, written: i64) -> Result<(), &'static str> {
    let refused = refused.trim();
    if refused.is_empty() || held.trim() != refused || written != 0 {
        return Err("refused quantity was not kept as a draft");
    }
    Ok(())
}

/// A crash or page-switch lookup keeps one effect. A second effect is not a re-login.
pub fn relogin_keeps_one_effect(decision: &str, effect: &str, effect_count: i64) -> Result<(), &'static str> {
    if !matches!(decision, "KEEP" | "LOOKUP") || effect.trim().is_empty() || effect_count != 1 {
        return Err("re-login does not keep one effect");
    }
    Ok(())
}

/// Restore stays inside the service address that held the draft. Another address is refused.
pub fn restore_same_service(stored: &str, requested: &str) -> Result<&'static str, &'static str> {
    if stored.trim().is_empty() || requested.trim().is_empty() {
        return Err("restore service is incomplete");
    }
    if stored != requested {
        return Err("restore belongs to another service");
    }
    Ok("RECHECKED")
}
