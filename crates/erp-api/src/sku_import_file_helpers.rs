use super::*;

pub(crate) fn resolve_import_file_type(
    file_type: Option<&str>,
    file_name: Option<&str>,
) -> Option<String> {
    if let Some(raw) = file_type {
        let normalized = raw.trim().to_uppercase();
        if matches!(normalized.as_str(), "CSV" | "XLS" | "XLSX") {
            return Some(normalized);
        }
    }
    if let Some(name) = file_name {
        let lower = name.trim().to_lowercase();
        if lower.ends_with(".csv") {
            return Some("CSV".to_string());
        }
        if lower.ends_with(".xls") || lower.ends_with(".xlsx") {
            return Some("XLSX".to_string());
        }
    }
    Some("CSV".to_string())
}

pub(crate) fn decode_base64_payload(
    raw: &str,
) -> Result<Vec<u8>, (StatusCode, Json<ErrorResponse>)> {
    let trimmed = raw.trim();
    let payload = trimmed
        .split_once("base64,")
        .map(|(_, content)| content)
        .unwrap_or(trimmed);
    general_purpose::STANDARD
        .decode(payload)
        .map_err(|_| bad_request("invalid base64 payload"))
}

pub(crate) fn write_sku_import_file(
    content: &[u8],
    tenant_id: TenantId,
    file_type: &str,
) -> Result<PathBuf, (StatusCode, Json<ErrorResponse>)> {
    let mut path = sku_import_base_dir();
    path.push(tenant_id.to_string());
    fs::create_dir_all(&path).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "internal_error".to_string(),
                message: "failed to create import directory".to_string(),
                details: None,
                request_id: None,
            }),
        )
    })?;
    let ext = if file_type.eq_ignore_ascii_case("CSV") {
        "csv"
    } else if file_type.eq_ignore_ascii_case("XLS") {
        "xls"
    } else {
        "xlsx"
    };
    path.push(format!("sku-import-{}.{}", uuid::Uuid::new_v4(), ext));
    fs::write(&path, content).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "internal_error".to_string(),
                message: "failed to write import file".to_string(),
                details: None,
                request_id: None,
            }),
        )
    })?;
    Ok(path)
}
