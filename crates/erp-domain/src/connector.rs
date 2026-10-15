//! External connector identity. A stored decision is not a live external call.

/// Credentials stay inside one tenant. An empty system or secret reference is refused.
pub fn admit_connector_credential(system_code: &str, secret_ref: &str) -> Result<(), &'static str> {
    if system_code.trim().is_empty() || secret_ref.trim().is_empty() {
        return Err("connector credential is incomplete");
    }
    Ok(())
}

/// A cursor, external object, command, and mapping version must all be present.
pub fn admit_connector_cursor(
    system_code: &str,
    cursor_text: &str,
    external_object_id: &str,
    command_id: &str,
    mapping_version: i32,
) -> Result<(), &'static str> {
    if system_code.trim().is_empty()
        || cursor_text.trim().is_empty()
        || external_object_id.trim().is_empty()
        || command_id.trim().is_empty()
        || mapping_version <= 0
    {
        return Err("connector cursor is incomplete");
    }
    Ok(())
}

/// An unknown external result must be queried before another business effect is posted.
pub fn external_result_decision(known: bool, queried: bool) -> Result<&'static str, &'static str> {
    if !known && !queried {
        return Err("an unknown external result must be queried first");
    }
    if known { Ok("POSTED") } else { Ok("QUERIED") }
}
