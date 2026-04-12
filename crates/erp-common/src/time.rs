//! Time parsing helpers for platform adapters.

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};

/// Parse a datetime string like `yyyy-MM-dd HH:mm:ss` in GMT+8 and convert to UTC.
pub fn parse_gmt8_datetime_to_utc(s: &str) -> Option<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok()?;
    let offset = FixedOffset::east_opt(8 * 3600)?;
    let dt = offset.from_local_datetime(&naive).single()?;
    Some(dt.with_timezone(&Utc))
}

/// Parse unix timestamp seconds into UTC.
pub fn parse_unix_seconds_to_utc(sec: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(sec, 0).single()
}

/// Parse a JSON value that could be:
/// - string datetime `yyyy-MM-dd HH:mm:ss`
/// - string integer seconds
/// - integer seconds
pub fn parse_platform_time(v: &serde_json::Value) -> Option<DateTime<Utc>> {
    match v {
        serde_json::Value::String(s) => {
            if let Some(dt) = parse_gmt8_datetime_to_utc(s) {
                return Some(dt);
            }
            if let Ok(sec) = s.parse::<i64>() {
                return parse_unix_seconds_to_utc(sec);
            }
            None
        }
        serde_json::Value::Number(n) => parse_unix_seconds_to_utc(n.as_i64()?),
        _ => None,
    }
}
