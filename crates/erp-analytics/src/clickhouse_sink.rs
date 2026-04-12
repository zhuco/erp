pub fn resolve_clickhouse_endpoint() -> Option<String> {
    std::env::var("ERP_CLICKHOUSE_URL")
        .ok()
        .or_else(|| std::env::var("CLICKHOUSE_URL").ok())
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn clickhouse_ready_from_env() -> (bool, Option<String>) {
    match resolve_clickhouse_endpoint() {
        Some(endpoint) => (true, Some(endpoint)),
        None => (false, None),
    }
}
