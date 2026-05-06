pub(crate) fn normalize_inbound_doc_type(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    match value.as_str() {
        "PURCHASE" | "RETURN" | "TRANSFER" | "OTHER" => Some(value),
        _ => None,
    }
}

pub(crate) fn normalize_inbound_status(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    match value.as_str() {
        "DRAFT" | "AUDITED" | "RECEIVING" | "PUTAWAY_DONE" | "CANCELLED" => Some(value),
        _ => None,
    }
}

pub(crate) fn normalize_warehouse_area_type(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    match value.as_str() {
        "STORAGE" | "PICKING" | "RECEIVING" | "SHIPPING" => Some(value),
        _ => None,
    }
}

pub(crate) fn normalize_warehouse_area_status(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    match value.as_str() {
        "ACTIVE" | "ENABLED" => Some("ACTIVE".to_string()),
        "DISABLED" | "INACTIVE" => Some("DISABLED".to_string()),
        _ => None,
    }
}

pub(crate) fn normalize_stock_count_strategy(raw: Option<&str>) -> Option<String> {
    let value = raw.unwrap_or("FULL").trim().to_ascii_uppercase();
    match value.as_str() {
        "FULL" | "RECENT_DAYS" | "BRAND" | "CATEGORY" => Some(value),
        _ => None,
    }
}

pub(crate) fn normalize_stock_count_dimension(raw: Option<&str>) -> Option<String> {
    let value = raw.unwrap_or("SKU").trim().to_ascii_uppercase();
    match value.as_str() {
        "SKU" | "LOCATION" => Some(value),
        _ => None,
    }
}

pub(crate) fn normalize_stock_count_scope_type(raw: Option<&str>) -> Option<String> {
    let value = raw.unwrap_or("FULL").trim().to_ascii_uppercase();
    match value.as_str() {
        "FULL" | "AREA" | "LOCATION" | "SKU" => Some(value),
        _ => None,
    }
}

pub(crate) fn normalize_stock_count_mode(raw: Option<&str>) -> Option<String> {
    let value = raw.unwrap_or("OPEN").trim().to_ascii_uppercase();
    match value.as_str() {
        "OPEN" | "BLIND" => Some(value),
        _ => None,
    }
}

pub(crate) fn normalize_stock_count_type(raw: Option<&str>) -> Option<String> {
    let value = raw.unwrap_or("DOCUMENT").trim().to_ascii_uppercase();
    match value.as_str() {
        "DOCUMENT" | "IMPORT" => Some(value),
        _ => None,
    }
}
