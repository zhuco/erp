//! HTTP-related helpers used across adapters.

use serde_json::Value;
use std::collections::BTreeMap;

/// Parse an `application/x-www-form-urlencoded` body.
///
/// This uses `url::form_urlencoded` which performs percent-decoding.
pub fn parse_form_urlencoded(body: &[u8]) -> BTreeMap<String, String> {
    url::form_urlencoded::parse(body)
        .into_owned()
        .collect::<BTreeMap<String, String>>()
}

/// Get header value from the stored headers JSON (case-insensitive name match).
///
/// `headers` is expected to be an object where each key is a header name and the
/// value is the header value as string.
pub fn header_get(headers: &Value, name: &str) -> Option<String> {
    let obj = headers.as_object()?;
    for (k, v) in obj {
        if k.eq_ignore_ascii_case(name) {
            if let Some(s) = v.as_str() {
                return Some(s.to_string());
            }
            return Some(v.to_string());
        }
    }
    None
}

/// Attempt to parse body as JSON.
pub fn parse_json(body: &[u8]) -> Option<Value> {
    serde_json::from_slice(body).ok()
}
