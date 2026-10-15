//! File exchange. A typed cell is not a formula, and a large file does not enter the browser cache.

/// CSV and XLSX are the mapped formats. A blank format is refused.
pub fn admit_file_format(format: &str) -> Result<&'static str, &'static str> {
    match format {
        "CSV" => Ok("CSV"),
        "XLSX" => Ok("XLSX"),
        _ => Err("file format is not registered"),
    }
}

/// A column map names the source and the field. An unmapped required field fails the precheck.
pub fn admit_column_map(source: &str, field: &str, required: bool) -> Result<&'static str, &'static str> {
    if source.trim().is_empty() && required {
        return Err("a required column is unmapped");
    }
    if field.trim().is_empty() {
        return Err("column map needs a field");
    }
    Ok("MAPPED")
}

/// A stored cell is readable as text. A formula prefix is not a stored export.
/// This is a stored cell read, not an opened spreadsheet file.
pub fn export_cell_readable(kind: &str, value: &str) -> Result<&'static str, &'static str> {
    let encoding = admit_cell_encoding(kind, value)?;
    if value.starts_with(['=', '+', '@', '-', '\t', '\r', '\n']) {
        return Err("formula injection is refused");
    }
    Ok(encoding)
}

/// Text, quantity, and money stay typed. A leading equals sign is a formula and is refused.
pub fn admit_cell_encoding(kind: &str, value: &str) -> Result<&'static str, &'static str> {
    if value.starts_with('=') || value.starts_with('+') || value.starts_with('@') {
        return Err("formula injection is refused");
    }
    match kind {
        "TEXT" | "CODE" => Ok("TEXT"),
        "QTY" => Ok("QTY"),
        "MONEY" => Ok("MONEY"),
        _ => Err("cell type is not registered"),
    }
}

/// A file over the UI bound becomes a backend task. The browser does not hold the rows.
pub fn file_delivery(rows: i64, bound: i64) -> Result<&'static str, &'static str> {
    if rows <= 0 || bound <= 0 {
        return Err("file delivery is incomplete");
    }
    if rows > bound { Ok("BACKEND_TASK") } else { Ok("DIRECT") }
}

/// Device print goes to an agent. A browser claim is not a device status.
/// Paper-out and offline cannot be stored as printed.
pub fn print_status(agent: bool, device_status: &str) -> Result<&'static str, &'static str> {
    if !agent {
        return Err("device print needs an agent");
    }
    match device_status {
        "QUEUED" => Ok("QUEUED"),
        "PRINTED" => Ok("PRINTED"),
        "FAILED" => Ok("FAILED"),
        "PAPER_OUT" | "OFFLINE" => Err("paper-out or offline cannot be stored as printed"),
        _ => Err("print status is not registered"),
    }
}

/// A stored print is readable only when the device status is the stored status.
/// Paper-out and offline are not a printed job. This is not a physical device run.
pub fn print_readable(status: &str, device: &str, document: &str) -> Result<(), &'static str> {
    if device.trim().is_empty() || document.trim().is_empty() {
        return Err("print read is incomplete");
    }
    if !matches!(status, "QUEUED" | "FAILED") && status != "PRINTED" {
        return Err("print status is not readable");
    }
    if matches!(status, "PAPER_OUT" | "OFFLINE") {
        return Err("paper-out or offline cannot be stored as printed");
    }
    Ok(())
}
