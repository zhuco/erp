//! Confirmed cell saves. Display rounding does not change the stored quantity.

/// A cell names its stable row and column. A blank identity is not an edit.
pub fn admit_cell(row_id: &str, column_id: &str) -> Result<(), &'static str> {
    if row_id.trim().is_empty() || column_id.trim().is_empty() || row_id == "0" {
        return Err("cell identity is incomplete");
    }
    Ok(())
}

/// A read-only column and an unknown unit are refused before save.
pub fn admit_cell_change(read_only: bool, unit: &str, confirmed: bool) -> Result<&'static str, &'static str> {
    if read_only {
        return Err("read-only column is not submitted");
    }
    if unit.trim().is_empty() {
        return Err("unit is invalid");
    }
    if !confirmed {
        return Err("an unconfirmed change is not submitted");
    }
    Ok("CONFIRMED")
}

/// A new quantity uses the six-digit scale. Display digits do not replace the stored micros.
pub fn stored_qty_micros(display: &str, micros: i64) -> Result<i64, &'static str> {
    if display.trim().is_empty() || micros <= 0 {
        return Err("quantity is incomplete");
    }
    let shown = display.trim().trim_end_matches('0').trim_end_matches('.');
    let scaled = if shown.contains('.') {
        let (whole, fraction) = shown.split_once('.').unwrap_or((shown, ""));
        if fraction.len() > 6 {
            return Err("quantity has more than six fractional digits");
        }
        let padded = format!("{fraction:0<6}");
        let whole_units = whole.parse::<i64>().map_err(|_| "quantity text is invalid")?;
        let fraction_units = padded.parse::<i64>().map_err(|_| "quantity text is invalid")?;
        whole_units
            .checked_mul(1_000_000)
            .and_then(|units| units.checked_add(fraction_units))
            .ok_or("quantity is outside the supported range")?
    } else {
        shown.parse::<i64>().map_err(|_| "quantity text is invalid")?.checked_mul(1_000_000).ok_or("quantity is outside the supported range")?
    };
    if scaled != micros {
        return Err("display rounding cannot replace the stored quantity");
    }
    Ok(micros)
}

/// One document saves all of its lines or none. A partial line is not an effect.
pub fn document_save_decision(line_count: i64, valid_count: i64) -> Result<&'static str, &'static str> {
    if line_count <= 0 || valid_count < 0 || valid_count > line_count {
        return Err("document save is incomplete");
    }
    if valid_count == line_count { Ok("ATOMIC") } else { Err("a partial document is not saved") }
}

/// A refused document keeps zero saved lines. The invalid line stays named.
pub fn refused_document_readable(decision: &str, saved_lines: i64, invalid_row: &str) -> Result<(), &'static str> {
    if decision != "REFUSED" || saved_lines != 0 || invalid_row.trim().is_empty() {
        return Err("refused document is not readable");
    }
    Ok(())
}

/// A batch returns one result per object. A conflict names the object and does not cover the others.
pub fn object_result(version: i64, conflict: bool) -> Result<&'static str, &'static str> {
    if version < 1 {
        return Err("object version is missing");
    }
    if conflict { Ok("CONFLICT") } else { Ok("SAVED") }
}

/// A conflict keeps the typed input. The stored body stays the saved version and is not replaced.
pub fn conflict_keeps_input(stored_body: &str, kept_input: &str, stored_version: i64) -> Result<(), &'static str> {
    if stored_body.trim().is_empty() || kept_input.trim().is_empty() || stored_version < 1 {
        return Err("conflict input is incomplete");
    }
    if stored_body == kept_input {
        return Err("conflict input matches the stored body");
    }
    Ok(())
}

/// A readable batch names one saved object and one conflicted object. Neither covers the other.
pub fn object_batch_located(saved_id: &str, saved: &str, conflict_id: &str, conflict: &str) -> Result<(), &'static str> {
    if saved_id.trim().is_empty() || conflict_id.trim().is_empty() || saved_id == conflict_id {
        return Err("object batch location is incomplete");
    }
    if saved != "SAVED" || conflict != "CONFLICT" {
        return Err("object batch does not locate both results");
    }
    Ok(())
}

/// A batch over the UI budget becomes an import task. It is not saved as a page edit.
pub fn over_budget_becomes_import(rows: i64, budget: i64) -> Result<&'static str, &'static str> {
    if rows <= 0 || budget <= 0 {
        return Err("edit budget is incomplete");
    }
    if rows > budget { Ok("IMPORT") } else { Ok("PAGE") }
}
