//! Header filtering and redaction for proxy requests.

use axum::http::{HeaderMap, HeaderValue};

/// Headers that should not be forwarded (hop-by-hop).
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "host",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

/// Headers whose values must not be persisted in trace records.
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "set-cookie2",
    "x-api-key",
    "x-amz-security-token",
];

/// Sensitive headers whose prefix may be kept for debugging.
const PREFIX_REDACTED_HEADERS: &[&str] = &["authorization", "x-api-key"];

/// Filter headers for proxy forwarding.
///
/// - Removes hop-by-hop headers.
/// - Optionally redacts sensitive header values.
pub fn filter_headers(headers: &HeaderMap, redact: bool) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (key, value) in headers {
        let key_lower = key.as_str().to_lowercase();
        if HOP_BY_HOP.contains(&key_lower.as_str()) {
            continue;
        }
        if redact && SENSITIVE_HEADERS.contains(&key_lower.as_str()) {
            let redacted = redact_value(&key_lower, value);
            if let Ok(v) = HeaderValue::from_str(&redacted) {
                out.insert(key.clone(), v);
            }
        } else {
            out.insert(key.clone(), value.clone());
        }
    }
    out
}

/// Redact a sensitive header value.
/// For authorization/x-api-key, keeps first 12 chars + "..." if long enough.
fn redact_value(key: &str, value: &HeaderValue) -> String {
    let value_str = value.to_str().unwrap_or("");
    if PREFIX_REDACTED_HEADERS.contains(&key) && value_str.len() > 12 {
        format!("{}...", &value_str[..12])
    } else {
        "***".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_removes_hop_by_hop() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("connection", HeaderValue::from_static("keep-alive"));
        headers.insert("transfer-encoding", HeaderValue::from_static("chunked"));

        let filtered = filter_headers(&headers, false);
        assert!(filtered.contains_key("content-type"));
        assert!(!filtered.contains_key("connection"));
        assert!(!filtered.contains_key("transfer-encoding"));
    }

    #[test]
    fn test_redacts_sensitive() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer secret-token"));
        headers.insert("x-api-key", HeaderValue::from_static("short"));
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let filtered = filter_headers(&headers, true);
        assert_eq!(
            filtered["authorization"].to_str().unwrap(),
            "Bearer secre..."
        );
        assert_eq!(filtered["x-api-key"].to_str().unwrap(), "***");
        assert_eq!(
            filtered["content-type"].to_str().unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_no_redact_when_disabled() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer secret"));

        let filtered = filter_headers(&headers, false);
        assert_eq!(filtered["authorization"].to_str().unwrap(), "Bearer secret");
    }
}
