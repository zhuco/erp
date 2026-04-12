pub mod clickhouse_sink;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsInfraHealth {
    pub analytics_backend: String,
    pub clickhouse_ready: bool,
    pub clickhouse_endpoint: Option<String>,
    pub sso_ready: bool,
    pub sso_issuer: Option<String>,
}

pub fn detect_analytics_infra_health() -> AnalyticsInfraHealth {
    let (clickhouse_ready, clickhouse_endpoint) = clickhouse_sink::clickhouse_ready_from_env();
    let sso_issuer = std::env::var("ERP_KEYCLOAK_ISSUER")
        .ok()
        .or_else(|| std::env::var("KEYCLOAK_ISSUER").ok())
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty());
    let sso_ready = sso_issuer.is_some();

    AnalyticsInfraHealth {
        analytics_backend: if clickhouse_ready {
            "CLICKHOUSE".to_string()
        } else {
            "POSTGRES".to_string()
        },
        clickhouse_ready,
        clickhouse_endpoint,
        sso_ready,
        sso_issuer,
    }
}
