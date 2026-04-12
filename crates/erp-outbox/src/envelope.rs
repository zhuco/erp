use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEnvelope {
    pub tenant_id: TenantId,
    pub outbox_id: uuid::Uuid,
    pub event_type: String,
    pub payload: Value,
    pub produced_at: DateTime<Utc>,
    pub trace_id: Option<String>,
}
