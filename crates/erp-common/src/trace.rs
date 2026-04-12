pub const TRACE_ID_HEADER: &str = "X-Trace-Id";
pub const REQUEST_ID_HEADER: &str = "X-Request-Id";

pub fn generate_trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
