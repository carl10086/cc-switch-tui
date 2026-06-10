use serde_json::Value;

const MAX_CLAUDE_SESSION_ID_LEN: usize = 128;

/// Extract `session_id` from `metadata.user_id` in the request body.
pub fn extract_claude_session_id(body: &Value) -> Option<String> {
    let user_id = body
        .get("metadata")?
        .get("user_id")?
        .as_str()?;

    let parsed: Value = serde_json::from_str(user_id).ok()?;
    let s = parsed.get("session_id")?.as_str()?;

    if s.len() > MAX_CLAUDE_SESSION_ID_LEN {
        return None;
    }
    Some(s.to_string())
}

/// Redact PII fields (`device_id`, `account_uuid`) from `metadata.user_id`.
/// Returns `true` if redaction was applied (or no action needed).
/// Returns `false` only if the user_id JSON could not be parsed.
pub fn redact_user_id_pii(body: &mut Value) -> bool {
    let Some(user_id_str) = body
        .get_mut("metadata")
        .and_then(|m| m.get_mut("user_id"))
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
    else {
        return true;
    };

    let Ok(mut parsed) = serde_json::from_str::<Value>(&user_id_str) else {
        return false;
    };

    if let Some(obj) = parsed.as_object_mut() {
        if obj.contains_key("device_id") {
            obj.insert("device_id".to_string(), Value::String("***".to_string()));
        }
        if obj.contains_key("account_uuid") {
            obj.insert("account_uuid".to_string(), Value::String("***".to_string()));
        }
    }

    let redacted = serde_json::to_string(&parsed).unwrap_or_else(|_| user_id_str);
    if let Some(metadata) = body.get_mut("metadata") {
        if let Some(user_id) = metadata.get_mut("user_id") {
            *user_id = Value::String(redacted);
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_success() {
        let body = json!({
            "metadata": {
                "user_id": "{\"device_id\":\"abc\",\"account_uuid\":\"uuid-1\",\"session_id\":\"88b40ca7\"}"
            }
        });
        assert_eq!(
            extract_claude_session_id(&body),
            Some("88b40ca7".to_string())
        );
    }

    #[test]
    fn test_extract_missing_metadata() {
        let body = json!({});
        assert_eq!(extract_claude_session_id(&body), None);
    }

    #[test]
    fn test_extract_missing_user_id() {
        let body = json!({"metadata": {}});
        assert_eq!(extract_claude_session_id(&body), None);
    }

    #[test]
    fn test_extract_user_id_not_string() {
        let body = json!({"metadata": {"user_id": 123}});
        assert_eq!(extract_claude_session_id(&body), None);
    }

    #[test]
    fn test_extract_malformed_json() {
        let body = json!({"metadata": {"user_id": "not-json"}});
        assert_eq!(extract_claude_session_id(&body), None);
    }

    #[test]
    fn test_extract_no_session_id() {
        let body = json!({"metadata": {"user_id": "{\"device_id\":\"abc\"}"}});
        assert_eq!(extract_claude_session_id(&body), None);
    }

    #[test]
    fn test_extract_session_id_too_long() {
        let long_id = "a".repeat(129);
        let body = json!({
            "metadata": {
                "user_id": format!("{{\"session_id\":\"{}\"}}", long_id)
            }
        });
        assert_eq!(extract_claude_session_id(&body), None);
    }

    #[test]
    fn test_extract_old_format_flat_slug() {
        let body = json!({"metadata": {"user_id": "flat-slug-value"}});
        assert_eq!(extract_claude_session_id(&body), None);
    }

    #[test]
    fn test_redact_strips_device_and_account_keeps_session() {
        let mut body = json!({
            "metadata": {
                "user_id": "{\"device_id\":\"abc\",\"account_uuid\":\"uuid-1\",\"session_id\":\"88b40ca7\"}"
            }
        });
        assert!(redact_user_id_pii(&mut body));
        let user_id = body["metadata"]["user_id"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(user_id).unwrap();
        assert_eq!(parsed["device_id"], "***");
        assert_eq!(parsed["account_uuid"], "***");
        assert_eq!(parsed["session_id"], "88b40ca7");
    }

    #[test]
    fn test_redact_no_user_id() {
        let mut body = json!({"metadata": {}});
        assert!(redact_user_id_pii(&mut body));
    }

    #[test]
    fn test_redact_flat_slug() {
        let mut body = json!({"metadata": {"user_id": "flat-slug"}});
        assert!(!redact_user_id_pii(&mut body));
        assert_eq!(body["metadata"]["user_id"], "flat-slug");
    }

    #[test]
    fn test_redact_partial_fields() {
        let mut body = json!({
            "metadata": {
                "user_id": "{\"device_id\":\"abc\",\"session_id\":\"s1\"}"
            }
        });
        assert!(redact_user_id_pii(&mut body));
        let user_id = body["metadata"]["user_id"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(user_id).unwrap();
        assert_eq!(parsed["device_id"], "***");
        assert_eq!(parsed["session_id"], "s1");
        assert!(!parsed.as_object().unwrap().contains_key("account_uuid"));
    }
}
